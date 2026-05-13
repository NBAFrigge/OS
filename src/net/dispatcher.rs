use alloc::borrow::ToOwned;

use crate::net::{
    arp,
    e1000::E1000_DRIVER,
    ethernet,
    interface::NETWORK_INTERFACE,
    ipv4::{ipv4_struct::ip_Header, transport::icmp::handle_icmp_packet},
};

pub fn poll_network() {
    x86_64::instructions::interrupts::without_interrupts(|| {
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
    });
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
                    let ip_header =
                        unsafe { &*(decoded_frame.payload.as_ptr() as *const ip_Header) };

                    let payload = &decoded_frame.payload[20..];

                    match ip_header.protocol {
                        1 => {
                            // ICMP
                            handle_icmp_packet(ip_header.src_ip, payload);
                        }
                        _ => {}
                    }
                }
                _ => {}
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
