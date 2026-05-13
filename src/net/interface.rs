use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::net::{arp::arp_struct::ArpPacket, ipv4::ipv4_struct::ip_Header};

pub struct Interface {
    pub hw_addr: Option<[u8; 6]>,
    pub ip_addr: Option<[u8; 4]>,
    pub subnet_mask: Option<[u8; 4]>,
    pub gateway_ip: Option<[u8; 4]>,
    tx_buffer: [u8; 1514],
    pub rx_queue: VecDeque<Vec<u8>>,
    pub tx_queue: VecDeque<Vec<u8>>,
}

impl Interface {
    pub fn is_local(&self, target_ip: [u8; 4]) -> bool {
        if let (Some(ip), Some(mask)) = (self.ip_addr, self.subnet_mask) {
            for i in 0..4 {
                if (ip[i] & mask[i]) != (target_ip[i] & mask[i]) {
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub fn send_ipv4(&mut self, mut header: ip_Header, payload: &[u8], target_mac: [u8; 6]) {
        header.calculate_checksum();

        let mut ip_payload = [0u8; 1480];
        let ip_size = header.serialize(payload, &mut ip_payload);

        self.send_ethernet(target_mac, [0x08, 0x00], &ip_payload[..ip_size]);
    }

    pub fn send_arp(&mut self, arp_packet: &ArpPacket) {
        let broadcast = [0xFF; 6];
        let arp_bytes = arp_packet.as_bytes();

        self.send_ethernet(broadcast, [0x08, 0x06], arp_bytes);
    }

    fn send_ethernet(&mut self, dest_mac: [u8; 6], ether_type: [u8; 2], payload: &[u8]) {
        let src_mac = self.hw_addr.expect("MAC not set");
        self.tx_buffer[0..6].copy_from_slice(&dest_mac);
        self.tx_buffer[6..12].copy_from_slice(&src_mac);
        self.tx_buffer[12..14].copy_from_slice(&ether_type);

        let payload_len = payload.len();
        self.tx_buffer[14..14 + payload_len].copy_from_slice(payload);

        let mut total_size = 14 + payload_len;

        if total_size < 60 {
            self.tx_buffer[total_size..60].fill(0);
            total_size = 60;
        }

        self.tx_queue
            .push_back(self.tx_buffer[..total_size].to_vec());
    }
}

lazy_static! {
    pub static ref NETWORK_INTERFACE: Mutex<Interface> = Mutex::new(Interface {
        hw_addr: None,
        ip_addr: None,
        subnet_mask: None,
        gateway_ip: None,
        tx_buffer: [0u8; 1514],
        rx_queue: VecDeque::new(),
        tx_queue: VecDeque::new()
    });
}
