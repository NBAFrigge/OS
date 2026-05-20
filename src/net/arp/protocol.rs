use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::net::{
    arp::arp_struct::{self, ArpPacket},
    interface::NETWORK_INTERFACE,
};
use crate::{kdebug, kwarn};

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
    let mut table = ArpTable.lock();

    if let Some(entry) = table.get(ip_target) {
        if entry.state == EntryState::SOLVED {
            return entry.mac_address;
        }
        return None;
    }

    table.insert(
        *ip_target,
        ArpTableEntry {
            mac_address: None,
            state: EntryState::PENDING,
        },
    );

    drop(table);

    kdebug!(
        "ARP: sending request for {}.{}.{}.{}",
        ip_target[0], ip_target[1], ip_target[2], ip_target[3]
    );

    let arp_packet = ArpPacket::new(
        super::arp_struct::ArpOperation::Request,
        sender_hw_address,
        sender_ip,
        [0; 6],
        *ip_target,
    );

    NETWORK_INTERFACE.lock().send_arp(&arp_packet);
    None
}

pub fn is_resolved(ip: &[u8; 4]) -> bool {
    let table = ArpTable.lock();
    table
        .get(ip)
        .map_or(false, |e| e.state == EntryState::SOLVED)
}

pub fn handle_packet(arp_packet: ArpPacket) {
    if arp_packet.operation == arp_struct::ArpOperation::Request as u16 {
        let (our_ip, our_mac) = {
            let iface = NETWORK_INTERFACE.lock();
            (iface.ip_addr, iface.hw_addr)
        };
        if let (Some(our_ip), Some(our_mac)) = (our_ip, our_mac) {
            if arp_packet.target_proto_addr == our_ip {
                kdebug!(
                    "ARP: reply to {}.{}.{}.{}",
                    arp_packet.sender_proto_addr[0], arp_packet.sender_proto_addr[1],
                    arp_packet.sender_proto_addr[2], arp_packet.sender_proto_addr[3]
                );
                let reply = ArpPacket::new(
                    arp_struct::ArpOperation::Reply,
                    our_mac,
                    our_ip,
                    arp_packet.sender_hw_addr,
                    arp_packet.sender_proto_addr,
                );
                NETWORK_INTERFACE.lock().send_arp_to(&reply, arp_packet.sender_hw_addr);
            }
        }
    } else if arp_packet.operation == arp_struct::ArpOperation::Reply as u16 {
        let ip = arp_packet.sender_proto_addr;
        kdebug!(
            "ARP: resolved {}.{}.{}.{} -> {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            ip[0], ip[1], ip[2], ip[3],
            arp_packet.sender_hw_addr[0], arp_packet.sender_hw_addr[1],
            arp_packet.sender_hw_addr[2], arp_packet.sender_hw_addr[3],
            arp_packet.sender_hw_addr[4], arp_packet.sender_hw_addr[5]
        );
        let mut table = ArpTable.lock();
        table.insert(
            arp_packet.sender_proto_addr,
            ArpTableEntry {
                mac_address: Some(arp_packet.sender_hw_addr),
                state: EntryState::SOLVED,
            },
        );
    } else {
        let op = arp_packet.operation;
        kwarn!("ARP: unknown operation {}", op);
    }
}

lazy_static! {
    pub static ref ArpTable: Mutex<BTreeMap<[u8; 4], ArpTableEntry>> = Mutex::new(BTreeMap::new());
}
