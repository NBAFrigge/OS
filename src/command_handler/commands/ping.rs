use crate::net::interface::NETWORK_INTERFACE;
use crate::net::ipv4::protocol;
use crate::net::ipv4::transport::icmp::IcmpPacket;
use alloc::vec::Vec;

const PAYLOAD: &[u8; 10] = b"SWAG ping!";

pub fn cmd_ping(args: &str) {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    if args_list.is_empty() {
        println!("Usage: ping <ip_address>");
        return;
    }

    let target_ip = parse_ip(args_list[0]);

    // Recuperiamo i dati dell'interfaccia
    let (my_ip, my_mac) = {
        let interface = NETWORK_INTERFACE.lock();
        let ip = interface.ip_addr.expect("IP not set");
        let mac = interface.hw_addr.expect("MAC not set");
        (ip, mac)
    };

    println!("PING {} ({:?})", args_list[0], target_ip);

    let ping_data = IcmpPacket::new_ping(0x1234, 1, PAYLOAD);

    protocol::send(my_ip, my_mac, target_ip, 1, &ping_data);
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
