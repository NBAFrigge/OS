use crate::ktrace;
use crate::net::{
    arp,
    e1000::E1000_DRIVER,
    ethernet,
    interface::NETWORK_INTERFACE,
    ipv4::{
        ipv4_struct::ip_Header,
        transport::{icmp::icmp::handle_icmp_packet, udp::socket::handle_udp_packet},
    },
};

pub fn poll_network() {
    if let Some(ref mut nic) = *E1000_DRIVER.lock() {
        let mut interface = NETWORK_INTERFACE.lock();
        while let Some(frame_bytes) = nic.receive() {
            interface.rx_queue.push_back(frame_bytes.to_vec());
        }

        while let Some(frame) = interface.tx_queue.pop_front() {
            if !nic.send(&frame) {
                interface.tx_queue.push_front(frame);
                break;
            }
        }
    }
    while !NETWORK_INTERFACE.lock().rx_queue.is_empty() {
        let frame_bytes = NETWORK_INTERFACE.lock().rx_queue.pop_front();
        if let Some(decoded_frame) = ethernet::parse(&frame_bytes.unwrap()) {
            match decoded_frame.ether_type {
                0x0806 => {
                    // ARP
                    if let Some(arp_packet) =
                        arp::arp_struct::ArpPacket::from_bytes(decoded_frame.payload)
                    {
                        arp::protocol::handle_packet(arp_packet);
                    }
                }
                0x0800 => {
                    // IPV4
                    if decoded_frame.payload.len() < 20 {
                        ktrace!("IPv4: frame too short ({} bytes)", decoded_frame.payload.len());
                        continue;
                    }

                    let ip_header = unsafe {
                        core::ptr::read_unaligned(decoded_frame.payload.as_ptr() as *const ip_Header)
                    };

                    let header_len = (ip_header.ihl() * 4) as usize;
                    let total_len = u16::from_be(ip_header.total_length) as usize;

                    if header_len < 20 {
                        ktrace!("IPv4: invalid IHL {}", ip_header.ihl());
                        continue;
                    }
                    if decoded_frame.payload.len() < header_len {
                        ktrace!("IPv4: payload shorter than IHL ({} < {})", decoded_frame.payload.len(), header_len);
                        continue;
                    }
                    if total_len < header_len || decoded_frame.payload.len() < total_len {
                        ktrace!("IPv4: invalid total_length {} (buf={}, hdr={})", total_len, decoded_frame.payload.len(), header_len);
                        continue;
                    }

                    let payload = &decoded_frame.payload[header_len..total_len];

                    let flags_fragment = u16::from_be(ip_header.flags_fragment);
                    let more_fragments = (flags_fragment & 0x2000) != 0;
                    let fragment_offset = flags_fragment & 0x1FFF;
                    if more_fragments || fragment_offset != 0 {
                        ktrace!("IPv4: fragmented packet dropped (flags={:#x} offset={})", flags_fragment >> 13, fragment_offset);
                        continue;
                    }

                    match ip_header.protocol {
                        1 => {
                            // ICMP
                            handle_icmp_packet(ip_header.src_ip, payload);
                        }
                        17 => {
                            // UDP
                            handle_udp_packet(&ip_header.src_ip, payload);
                        }
                        _ => {
                            ktrace!("IPv4: unknown protocol {}", ip_header.protocol);
                        }
                    }
                }
                _ => {
                    ktrace!(
                        "Ethernet: unknown ethertype 0x{:04X}",
                        decoded_frame.ether_type
                    );
                }
            }
        }
    }
}

pub extern "C" fn network_task_entry() {
    loop {
        poll_network();
        crate::task::task::yield_now();
    }
}
