use core::sync::atomic::{AtomicU16, Ordering};

use crate::kdebug;
use crate::net::{arp, interface::NETWORK_INTERFACE, ipv4::ipv4_struct::ip_Header};

pub fn send(dst_ip: [u8; 4], protocol: u8, payload: &[u8]) {
    let id = IP_PACKET_ID.fetch_add(1, Ordering::SeqCst);

    let (src_ip, src_mac, target_ip_for_arp, is_broadcast) = {
        let interface = NETWORK_INTERFACE.lock();

        let src_ip = interface.ip_addr.unwrap_or([0, 0, 0, 0]);
        let src_mac = interface.hw_addr.expect("MAC not set");

        let is_broadcast = dst_ip == [255, 255, 255, 255];

        let target = if is_broadcast {
            dst_ip
        } else if interface.is_local(dst_ip) {
            dst_ip
        } else {
            interface.gateway_ip.expect("Gateway IP not configured")
        };

        (src_ip, src_mac, target, is_broadcast)
    };

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

    if is_broadcast {
        let broadcast_mac = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        NETWORK_INTERFACE
            .lock()
            .send_ipv4(header, payload, broadcast_mac);
    } else {
        if let Some(mac) = arp::protocol::resolve_mac(&target_ip_for_arp, src_ip, src_mac) {
            NETWORK_INTERFACE.lock().send_ipv4(header, payload, mac);
        } else {
            kdebug!(
                "IPv4: ARP pending for {}.{}.{}.{}, packet dropped",
                target_ip_for_arp[0],
                target_ip_for_arp[1],
                target_ip_for_arp[2],
                target_ip_for_arp[3]
            );
        }
    }
}
pub static IP_PACKET_ID: AtomicU16 = AtomicU16::new(0);
