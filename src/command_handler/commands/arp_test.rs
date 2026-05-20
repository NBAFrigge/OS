use alloc::vec::Vec;

use crate::{
    command_handler::commands::helpers::parse_ip,
    net::{arp, interface::NETWORK_INTERFACE},
};

pub fn cmd_arp_test(raw_args: &str) {
    let args: Vec<&str> = raw_args.split_whitespace().collect();

    if args.is_empty() {
        println!("Usage: arptest <ip_address>");
        return;
    }

    let Some(target_ip) = parse_ip(args[0]) else {
        println!("arptest: invalid ip address");
        return;
    };

    let (sender_hw_addr, sender_ip) = {
        let interface = NETWORK_INTERFACE.lock();
        (interface.hw_addr.unwrap(), interface.ip_addr.unwrap())
    };

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
