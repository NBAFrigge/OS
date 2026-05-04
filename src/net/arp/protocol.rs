use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::net::{arp::arp::ArpPacket, e1000::E1000_DRIVER, ethernet};

#[derive(PartialEq)]
pub enum EntryState {
    SOLVED,
    PENDING,
}

pub struct ArpTableEntry {
    pub mac_address: Option<[u8; 6]>,
    pub state: EntryState,
}

pub fn resolve_mac(
    ip_target: &[u8; 4],
    sender_ip: [u8; 4],
    sender_hw_address: [u8; 6],
) -> Option<[u8; 6]> {
    {
        let table = ArpTable.lock();
        if let Some(entry) = table.get(ip_target) {
            if entry.state == EntryState::SOLVED {
                return entry.mac_address;
            }
            return None;
        }
    }

    let arp_packet = ArpPacket::new(
        super::arp::ArpOperation::Request,
        sender_hw_address,
        sender_ip,
        [0, 0, 0, 0, 0, 0],
        *ip_target,
    );

    let arp_ethernet_frag = ethernet::build(
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        sender_hw_address,
        0x0806,
        arp_packet.as_bytes(),
    );

    serial_println!("DEBUG: Attempting to send ARP...");

    if let Some(ref mut nic) = *E1000_DRIVER.lock() {
        nic.send(&arp_ethernet_frag);
    }

    {
        let mut table = ArpTable.lock();
        table.insert(
            *ip_target,
            ArpTableEntry {
                mac_address: None,
                state: EntryState::PENDING,
            },
        );
    }

    None
}

pub fn handle_packet(arp_packet: ArpPacket) {
    serial_println!("DEBUG: Received ARP packet");
    if arp_packet.operation == 2 {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut table = ArpTable.lock();
            table.insert(
                arp_packet.sender_proto_addr,
                ArpTableEntry {
                    mac_address: Some(arp_packet.sender_hw_addr),
                    state: EntryState::SOLVED,
                },
            );
        });
    }
}

lazy_static! {
    pub static ref ArpTable: Mutex<BTreeMap<[u8; 4], ArpTableEntry>> = Mutex::new(BTreeMap::new());
}
