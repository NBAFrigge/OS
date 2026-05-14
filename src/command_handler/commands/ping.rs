use core::sync::atomic::Ordering;

use alloc::vec::Vec;

use crate::{
    idt::interrupt::TICKS,
    net::{
        arp,
        interface::NETWORK_INTERFACE,
        ipv4::{
            protocol,
            transport::{
                icmp::icmp::{IcmpPacket, PING_REPLY},
                udp::dns::solver::DnsResolver,
            },
        },
    },
    task::task::sleep,
};

pub fn cmd_ping(args: &str) {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    if args_list.is_empty() {
        println!("Usage: ping <hostname/ip>");
        return;
    }

    let target_ip = match resolve_target(args_list[0]) {
        Some(ip) => ip,
        None => {
            println!("ping: unknown host {}", args_list[0]);
            return;
        }
    };

    let target_ip_for_arp = {
        let interface = NETWORK_INTERFACE.lock();
        if interface.is_local(target_ip) {
            target_ip
        } else {
            interface.gateway_ip.expect("Gateway not configured")
        }
    };

    println!(
        "PING {} ({}.{}.{}.{})",
        args_list[0], target_ip[0], target_ip[1], target_ip[2], target_ip[3]
    );

    if !arp::protocol::is_resolved(&target_ip_for_arp) {
        protocol::send(target_ip, 1, &[]);
        let mut resolved = false;
        for _ in 0..50 {
            sleep(10);
            if arp::protocol::is_resolved(&target_ip_for_arp) {
                resolved = true;
                break;
            }
        }
        if !resolved {
            println!("ping: ARP timeout");
            return;
        }
    }

    let payload = TICKS.load(Ordering::Relaxed);
    let payload_bytes = payload.to_ne_bytes();
    let ping_data = IcmpPacket::new_ping(0x1234, 1, &payload_bytes);

    *PING_REPLY.lock() = None;
    protocol::send(target_ip, 1, &ping_data);

    let send_time = TICKS.load(Ordering::Relaxed);

    for _ in 0..500 {
        sleep(10);
        if let Some((reply_ip, seq)) = PING_REPLY.lock().take() {
            let rtt = TICKS.load(Ordering::Relaxed) - send_time;
            println!(
                "64 bytes from {}.{}.{}.{}: icmp_seq={} ttl=64 time={} ms",
                reply_ip[0], reply_ip[1], reply_ip[2], reply_ip[3], seq, rtt
            );
            return;
        }
    }

    println!("Request timeout");
}

fn resolve_target(target: &str) -> Option<[u8; 4]> {
    if let Some(ip) = parse_ip(target) {
        return Some(ip);
    }
    DnsResolver.get(target)
}

fn parse_ip(ip_str: &str) -> Option<[u8; 4]> {
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
