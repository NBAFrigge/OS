use crate::net::{arp, interface::NETWORK_INTERFACE, ipv4::ipv4_struct::ip_Header};

pub fn send(src_ip: [u8; 4], src_mac_addr: [u8; 6], dst_ip: [u8; 4], protocol: u8, payload: &[u8]) {
    let mut header = ip_Header {
        version_ihl: (4 << 4) | 5,
        tos: 0,
        total_length: ((20 + payload.len()) as u16).to_be(),
        id: 0, //TODO: implment counter
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
        interface.gateway_ip.expect("Gateway IP non configurato!")
    };

    let mac_addr = arp::protocol::resolve_mac(&target_ip_for_arp, src_ip, src_mac_addr);
    match mac_addr {
        Some(mac_addr) => {
            //TODO: finish with the frame building and sending
        }
        None => {
            print!("ARP still resolving the mac addr")
        }
    }
}
