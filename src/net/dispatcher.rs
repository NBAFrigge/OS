use crate::net::{arp, e1000::E1000_DRIVER, ethernet, ipv4::ipv4_struct::ip_Header};

pub fn poll_network() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(ref mut nic) = *E1000_DRIVER.lock() {
            while let Some(frame_bytes) = nic.receive() {
                serial_println!("Debug packet arrived");
                if let Some(decoded_frame) = ethernet::parse(&frame_bytes) {
                    match decoded_frame.ether_type {
                        0x0806 => {
                            if let Some(arp_packet) =
                                arp::arp_struct::ArpPacket::from_bytes(decoded_frame.payload)
                            {
                                serial_println!("packet arp arrived");
                                arp::protocol::handle_packet(arp_packet);
                            }
                        }
                        0x0800 => {
                            let ip_header =
                                unsafe { &*(decoded_frame.payload.as_ptr() as *const ip_Header) };

                            let payload = &decoded_frame.payload[20..];

                            match ip_header.protocol {
                                1 => { // ICMP
                                     //TODO: implement icmp handling
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

pub extern "C" fn network_task_entry() {
    loop {
        poll_network();
        crate::task::task::yield_now();
    }
}
