use core::sync::atomic::Ordering;

use alloc::vec::Vec;

use crate::{
    command_handler::commands::helpers::parse_ip,
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
    timer::tsc,
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
    crate::net::dispatcher::flush_tx();

    let send_time = tsc::now_us();

    loop {
        if let Some((reply_ip, seq)) = PING_REPLY.lock().take() {
            let rtt = tsc::now_us() - send_time;
            println!(
                "64 bytes from {}.{}.{}.{}: icmp_seq={} ttl=64 time={} us",
                reply_ip[0], reply_ip[1], reply_ip[2], reply_ip[3], seq, rtt
            );
            return;
        }
        x86_64::instructions::interrupts::enable_and_hlt();
        if tsc::now_us() - send_time > 1_000_000 {
            break;
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
