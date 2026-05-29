use alloc::{str, vec::Vec};

use crate::{
    kerror,
    net::ipv4::transport::{tcp::socket, udp::dns::solver::DnsResolver},
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
