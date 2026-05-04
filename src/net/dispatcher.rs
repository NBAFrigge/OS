use crate::net::{arp, e1000::E1000_DRIVER, ethernet};

pub fn poll_network() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(ref mut nic) = *E1000_DRIVER.lock() {
            while let Some(frame_bytes) = nic.receive() {
                if let Some(decoded_frame) = ethernet::parse(&frame_bytes) {
                    match decoded_frame.ether_type {
                        0x0806 => {
                            if let Some(arp_packet) =
                                arp::arp::ArpPacket::from_bytes(decoded_frame.payload)
                            {
                                arp::protocol::handle_packet(arp_packet);
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
    }
}
