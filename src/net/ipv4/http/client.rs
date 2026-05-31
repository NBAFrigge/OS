use alloc::{str, vec::Vec};

use crate::{
    command_handler::commands::sleep,
    idt::interrupt::TICKS,
    kdebug, kerror,
    net::ipv4::{
        http::{
            request::Request,
            response::{parse_response, response},
        },
        transport::{
            tcp::socket::{self, TcpState, TcpTuple, TCP_SOCKET_MANAGER},
            udp::dns::solver::DnsResolver,
        },
    },
    task::task::sleep,
};

pub struct connection<'a> {
    host: &'a str,
    remote_ip: [u8; 4],
    remote_port: u16,
    local_port: u16,
}

impl<'a> connection<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        let ip = resolve_target(host).unwrap();
        Self {
            host,
            remote_ip: ip,
            remote_port: port,
            local_port: 0,
        }
    }

    pub fn connect(&mut self) -> Result<(), &'static str> {
        self.local_port = socket::connect(&self.remote_ip, self.remote_port);
        if self.local_port == 0 {
            kerror!("tcp connection error");
            return Err("connection error");
        }
        Ok(())
    }

    //TODO: add timeout
    pub fn send(&self, request: Request) -> Option<response> {
        let tuple = TcpTuple {
            local_port: self.local_port,
            remote_ip: self.remote_ip,
            remote_port: self.remote_port,
        };

        let socket = {
            let manager = TCP_SOCKET_MANAGER.lock();
            manager.get(&tuple)?.clone()
        };

        while socket.lock().state != TcpState::Established {
            kdebug!("HTTP send: waiting socket {} opening", self.local_port);
            sleep(10);
        }

        let raw_request = request.build()?;

        socket.lock().write(&raw_request);

        //TODO: finish timeout
        let start_ticks = TICKS.load(core::sync::atomic::Ordering::Relaxed);

        kdebug!("HTTP: {} bytes sent", raw_request.len());

        let mut raw_response = [0u8; 65536];
        let mut offset = 0;
        while raw_response[..offset]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            == None
        {
            offset += socket.lock().read(&mut raw_response[offset..]);
            if (TICKS.load(core::sync::atomic::Ordering::Relaxed) - start_ticks) as usize
                > request.timeout
            {
                kdebug!("HTTP: request timeout");
                return None;
            }
        }
        kdebug!("HTTP: header_lenght found at {}", offset);

        let header_lenght = raw_response[..offset]
            .windows(4)
            .position(|w| w == b"\r\n\r\n")?
            + 4;
        let header_string = core::str::from_utf8(&raw_response[..header_lenght]).ok()?;

        let mut content_lenght = 0;
        let mut keep_alive = false;

        for s in header_string.split("\r\n") {
            if s.contains("Content-Length") {
                let splitted = s.split_once(":")?;
                content_lenght = splitted.1.trim().parse::<usize>().ok()?;
            }

            if s.contains("Connection") {
                let splitted = s.split_once(":")?;
                keep_alive = splitted.1.trim() == "keep_alive";
            }
        }
        kdebug!("HTTP: content_lenght: {}", content_lenght);

        while offset < content_lenght + header_lenght {
            offset += socket.lock().read(&mut raw_response[offset..]);
            if (TICKS.load(core::sync::atomic::Ordering::Relaxed) - start_ticks) as usize
                > request.timeout
            {
                kdebug!("HTTP: request timeout");
                return None;
            }
        }
        kdebug!("HTTP: request completed {}", offset);

        if !keep_alive {
            socket.lock().close();
        }

        parse_response(&raw_response)
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
