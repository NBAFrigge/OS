use alloc::{str, vec, vec::Vec};
use core::sync::atomic::Ordering;
use x86_64::instructions::interrupts;

use crate::{
    idt::interrupt::TICKS,
    kdebug, kerror,
    net::{
        interface::NETWORK_INTERFACE,
        ipv4::{
            http::{
                constants::HttpError,
                request::Request,
                response::{parse_response, response},
            },
            transport::{
                tcp::socket::{
                    self, try_send, TcpState, TcpTuple, LAST_TX_SEGMENT_TICK, TCP_SOCKET_MANAGER,
                },
                udp::{dhcp::packet::constants::DHCP_MAGIC_COOKIE, dns::solver::DnsResolver},
            },
        },
    },
    task::task::sleep,
};

const CONN_TIMEOUT: u64 = 300;

pub struct connection<'a> {
    host: &'a str,
    pub remote_ip: [u8; 4],
    remote_port: u16,
    pub local_port: u16,
    pub force_keep_alive: bool,
}

impl<'a> connection<'a> {
    pub fn new(host: &'a str, port: u16, force_ka: bool) -> Result<Self, HttpError> {
        let ip = resolve_target(host).ok_or(HttpError::DnsError)?;
        Ok(Self {
            host,
            remote_ip: ip,
            remote_port: port,
            local_port: 0,
            force_keep_alive: force_ka,
        })
    }

    pub fn new_with_ip(host: &'a str, ip: [u8; 4], port: u16, force_ka: bool) -> Self {
        Self {
            host,
            remote_ip: ip,
            remote_port: port,
            local_port: 0,
            force_keep_alive: force_ka,
        }
    }

    pub fn connect(&mut self) -> Result<(), HttpError> {
        self.local_port = socket::connect(&self.remote_ip, self.remote_port);
        if self.local_port == 0 {
            kerror!("tcp connection error");
            return Err(HttpError::ConnectionFailed);
        }
        let tuple = TcpTuple {
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        };

        let socket = {
            let manager = TCP_SOCKET_MANAGER.lock();
            manager
                .get(&tuple)
                .ok_or(HttpError::SocketNotFound)?
                .clone()
        };

        let t_start = TICKS.load(Ordering::Relaxed);
        loop {
            interrupts::disable();
            let established = socket.lock().state == TcpState::Established;
            if established {
                interrupts::enable();
                break;
            }
            interrupts::enable_and_hlt();

            if TICKS.load(Ordering::Relaxed) - t_start >= CONN_TIMEOUT {
                TCP_SOCKET_MANAGER.lock().remove(&tuple);
                return Err(HttpError::ConnectionFailed);
            }
        }
        Ok(())
    }

    pub fn close(&self) {
        let tuple = TcpTuple {
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        };
        if let Some(socket) = TCP_SOCKET_MANAGER.lock().get(&tuple).cloned() {
            let _ = socket.lock().close();
        }
    }

    pub fn is_open(&self) -> bool {
        let tuple = TcpTuple {
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        };
        if let Some(socket) = TCP_SOCKET_MANAGER.lock().get(&tuple).cloned() {
            return socket.lock().state == TcpState::Established;
        }
        false
    }

    pub fn send(&self, request: Request) -> Result<response, HttpError> {
        let t_start = TICKS.load(core::sync::atomic::Ordering::Relaxed);

        let tuple = TcpTuple {
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        };

        let socket = {
            let manager = TCP_SOCKET_MANAGER.lock();
            manager
                .get(&tuple)
                .ok_or(HttpError::SocketNotFound)?
                .clone()
        };

        let mut retry = 0;
        while socket.lock().state != TcpState::Established {
            sleep(11);
            retry += 1;
            if retry >= 20 {
                return Err(HttpError::ConnectionFailed);
            }
        }

        let raw_request = request.build().ok_or(HttpError::ParseError)?;
        let src_ip = NETWORK_INTERFACE.lock().ip_addr.unwrap_or([0, 0, 0, 0]);

        {
            let mut s = socket.lock();
            s.write(&raw_request);
            try_send(&mut s, &src_ip);
        }

        let start_ticks = TICKS.load(Ordering::Relaxed);

        let mut raw_response = vec![0u8; 65536];
        let mut offset = 0;
        let mut header_lenght = 0;
        let mut first_byte_tick: u64 = 0;
        loop {
            interrupts::disable();
            let prev = offset;
            let n = socket.lock().read(&mut raw_response[offset..]);
            if n > 0 && first_byte_tick == 0 {
                first_byte_tick = TICKS.load(Ordering::Relaxed);
            }
            offset += n;
            if n == 0 {
                interrupts::enable_and_hlt();
            } else {
                interrupts::enable();
            }
            if (TICKS.load(Ordering::Relaxed) - start_ticks) as usize > request.timeout {
                kdebug!("HTTP: request timeout");
                return Err(HttpError::Timeout);
            }
            let search_from = prev.saturating_sub(3);
            if let Some(pos) = raw_response[search_from..offset]
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
            {
                header_lenght = search_from + pos + 4;
                break;
            }
        }

        let header_string = core::str::from_utf8(&raw_response[..header_lenght])
            .map_err(|_| HttpError::ParseError)?;

        let mut content_lenght = 0;
        let mut keep_alive = false;

        for s in header_string.split("\r\n") {
            if s.contains("Content-Length") {
                let splitted = s.split_once(":").ok_or(HttpError::ParseError)?;
                content_lenght = splitted
                    .1
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .ok_or(HttpError::ParseError)?;
            }

            if s.contains("Connection") {
                let splitted = s.split_once(":").ok_or(HttpError::ParseError)?;
                keep_alive = splitted.1.trim() == "keep-alive";
            }
        }

        while offset < content_lenght + header_lenght {
            offset += socket.lock().read(&mut raw_response[offset..]);
            if (TICKS.load(Ordering::Relaxed) - start_ticks) as usize > request.timeout {
                kdebug!("HTTP: request timeout");
                return Err(HttpError::Timeout);
            }
        }
        let resp = parse_response(&raw_response[..offset]).ok_or(HttpError::ParseError)?;

        if !keep_alive && !self.force_keep_alive {
            socket.lock().close();
        }

        return Ok(resp);
    }
}

fn resolve_target(target: &str) -> Option<[u8; 4]> {
    if let Some(ip) = parse_ip(target) {
        return Some(ip);
    }
    DnsResolver.get(target)
}

pub fn parse_ip(ip_str: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = ip_str.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut ip = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        ip[i] = part.parse::<u8>().ok()?;
    }
    Some(ip)
}
