use core::sync::atomic::{AtomicU16, Ordering};

use crate::net::{arp, interface::NETWORK_INTERFACE, ipv4::ipv4_struct::ip_Header};

pub fn send(src_ip: [u8; 4], src_mac_addr: [u8; 6], dst_ip: [u8; 4], protocol: u8, payload: &[u8]) {
    let id = IP_PACKET_ID.fetch_add(1, Ordering::SeqCst);
    let mut header = ip_Header {
        version_ihl: (4 << 4) | 5,
        tos: 0,
        total_length: ((20 + payload.len()) as u16).to_be(),
        id: id.to_be(),
        flags_fragment: 0,
        ttl: 64,
        protocol,
        checksum: 0,
        src_ip,
        dst_ip,
    };

    header.calculate_checksum();

    let interface = NETWORK_INTERFACE.lock();
    let target_ip_for_arp = if interface.is_local(dst_ip) {
        dst_ip
    } else {
        interface.gateway_ip.expect("Gateway IP not configurated")
    };

    let mac_addr = arp::protocol::resolve_mac(&target_ip_for_arp, src_ip, src_mac_addr);
    match mac_addr {
        Some(mac_addr) => {
            NETWORK_INTERFACE
                .lock()
                .send_ipv4(header, payload, mac_addr);
        }
        None => {
            print!("ARP still resolving the mac addr")
        }
    }
}

pub static IP_PACKET_ID: AtomicU16 = AtomicU16::new(0);
