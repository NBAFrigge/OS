use alloc::vec::Vec;

use crate::net::arp;
use crate::net::interface::NETWORK_INTERFACE;
use crate::net::ipv4::protocol;
use crate::net::ipv4::transport::icmp::IcmpPacket;
use crate::task::task::sleep;

const PAYLOAD: &[u8; 10] = b"SWAG ping!";

pub fn cmd_ping(args: &str) {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    if args_list.is_empty() {
        println!("Usage: ping <ip_address>");
        return;
    }

    let target_ip = parse_ip(args_list[0]);

    let target_ip_for_arp = {
        let interface = NETWORK_INTERFACE.lock();
        if interface.is_local(target_ip) {
            target_ip
        } else {
            interface.gateway_ip.expect("Gateway not configured")
        }
    };

    println!("PING {}", args_list[0]);

    let ping_data = IcmpPacket::new_ping(0x1234, 1, PAYLOAD);

    protocol::send(target_ip, 1, &ping_data);

    if arp::protocol::is_resolved(&target_ip_for_arp) {
        return;
    }

    for _ in 0..10 {
        sleep(10);
        if arp::protocol::is_resolved(&target_ip_for_arp) {
            protocol::send(target_ip, 1, &ping_data);
            return;
        }
    }

    println!("ping: ARP timeout");
}

fn parse_ip(ip_str: &str) -> [u8; 4] {
    let mut ip = [0u8; 4];
    let parts: Vec<&str> = ip_str.split('.').collect();
    for i in 0..4 {
        if i < parts.len() {
            ip[i] = parts[i].parse().unwrap_or(0);
        }
    }
    ip
}
