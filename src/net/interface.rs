use lazy_static::lazy_static;
use spin::Mutex;

use crate::net::{arp::arp_struct::ArpPacket, e1000::E1000_DRIVER, ipv4::ipv4_struct::ip_Header};

pub struct Interface {
    pub hw_addr: Option<[u8; 6]>,
    pub ip_addr: Option<[u8; 4]>,
    pub subnet_mask: Option<[u8; 4]>,
    pub gateway_ip: Option<[u8; 4]>,
    tx_buffer: [u8; 1514],
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

        let src_mac = self.hw_addr.expect("MAC not set");
        self.tx_buffer[0..6].copy_from_slice(&target_mac);
        self.tx_buffer[6..12].copy_from_slice(&src_mac);
        self.tx_buffer[12..14].copy_from_slice(&[0x08, 0x00]);

        let ip_data_size = header.serialize(payload, &mut self.tx_buffer[14..]);

        let total_size = 14 + ip_data_size;

        if let Some(ref mut nic) = *E1000_DRIVER.lock() {
            nic.send(&self.tx_buffer[..total_size]);
        }
    }

    pub fn send_arp(&mut self, arp_packet: &ArpPacket) {
        let broadcast = [0xFF; 6];
        let src_mac = self.hw_addr.expect("MAC not set");

        self.tx_buffer[0..6].copy_from_slice(&broadcast);
        self.tx_buffer[6..12].copy_from_slice(&src_mac);
        self.tx_buffer[12..14].copy_from_slice(&[0x08, 0x06]); // EtherType ARP

        let arp_bytes = arp_packet.as_bytes();
        let size = arp_bytes.len();
        self.tx_buffer[14..14 + size].copy_from_slice(arp_bytes);

        if let Some(ref mut nic) = *E1000_DRIVER.lock() {
            nic.send(&self.tx_buffer[..14 + size]);
        }
    }
}

lazy_static! {
    pub static ref NETWORK_INTERFACE: Mutex<Interface> = Mutex::new(Interface {
        hw_addr: None,
        ip_addr: None,
        subnet_mask: None,
        gateway_ip: None,
        tx_buffer: [0u8; 1514],
    });
}
