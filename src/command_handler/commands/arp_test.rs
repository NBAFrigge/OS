use alloc::vec::Vec;

use crate::net::{
    arp::{self},
    e1000::E1000_DRIVER,
};

pub fn cmd_arp_test(raw_args: &str) {
    let args: Vec<&str> = raw_args.split_whitespace().collect();

    if args.len() < 1 {
        println!("Usage: arptest <ip_address>");
        return;
    }

    let target_ip = parse_ip(args[0]);
    let sender_hw_addr = if let Some(ref nic) = *E1000_DRIVER.lock() {
        nic.mac
    } else {
        println!("Error: NIC not initialized");
        return;
    };

    let sender_ip = [192, 168, 1, 231];

    println!("Resolving MAC for {}...", args[0]);

    match arp::protocol::resolve_mac(&target_ip, sender_ip, sender_hw_addr) {
        Some(mac) => {
            println!(
                "MAC found: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            );
        }
        None => {
            println!("ARP Request sent")
        }
    }
}

fn parse_ip(ip_str: &str) -> [u8; 4] {
    let mut ip = [0u8; 4];
    let parts: Vec<&str> = ip_str.split('.').collect();
    for i in 0..4 {
        ip[i] = parts[i].parse().unwrap_or(0);
    }
    ip
}
