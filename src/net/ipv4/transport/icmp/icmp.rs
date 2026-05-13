use crate::{net::ipv4::protocol::send, vgadriver::writer::WRITER};
use alloc::vec::Vec;
use core::ptr::read_unaligned;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref PING_REPLY: Mutex<Option<([u8; 4], u16)>> = Mutex::new(None);
}

pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct IcmpPacket {
    pub typ: u8,
    pub code: u8,
    pub checksum: u16,
    pub id: u16,
    pub sequence: u16,
}

impl IcmpPacket {
    pub fn new_ping(id: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
        let header = IcmpPacket {
            typ: ICMP_TYPE_ECHO_REQUEST,
            code: 0,
            checksum: 0,
            id: id.to_be(),
            sequence: seq.to_be(),
        };

        let mut packet_data = Vec::with_capacity(8 + payload.len());
        let header_ptr = &header as *const Self as *const u8;
        let header_slice = unsafe { core::slice::from_raw_parts(header_ptr, 8) };

        packet_data.extend_from_slice(header_slice);
        packet_data.extend_from_slice(payload);

        let checksum = Self::calculate_checksum_raw(&packet_data);
        packet_data[2] = (checksum >> 8) as u8;
        packet_data[3] = (checksum & 0xFF) as u8;

        packet_data
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                (self as *const IcmpPacket) as *const u8,
                core::mem::size_of::<IcmpPacket>(),
            )
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<IcmpPacket>() {
            return None;
        }
        let packet = unsafe { read_unaligned(data.as_ptr() as *const IcmpPacket) };
        Some(Self {
            typ: packet.typ,
            code: packet.code,
            checksum: u16::from_be(packet.checksum),
            id: u16::from_be(packet.id),
            sequence: u16::from_be(packet.sequence),
        })
    }

    pub fn calculate_checksum_raw(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        for chunk in data.chunks_exact(2) {
            sum += ((chunk[0] as u16) << 8 | chunk[1] as u16) as u32;
        }
        if data.len() % 2 != 0 {
            sum += (data[data.len() - 1] as u32) << 8;
        }
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !(sum as u16)
    }
}

pub fn handle_icmp_packet(src_ip: [u8; 4], payload: &[u8]) {
    if let Some(icmp) = IcmpPacket::from_bytes(payload) {
        match icmp.typ {
            ICMP_TYPE_ECHO_REPLY => {
                *PING_REPLY.lock() = Some((src_ip, icmp.sequence));
            }
            ICMP_TYPE_ECHO_REQUEST => {
                print!("\n");
                println!("ICMP: Echo Request receveid from {:?}", src_ip);
                x86_64::instructions::interrupts::without_interrupts(|| {
                    WRITER.lock().redraw_shell_line();
                });

                reply_to_ping(src_ip, icmp, &payload[8..]);
            }
            _ => {}
        }
    }
}

pub fn reply_to_ping(target_ip: [u8; 4], request: IcmpPacket, data: &[u8]) {
    let reply = IcmpPacket {
        typ: ICMP_TYPE_ECHO_REPLY,
        code: 0,
        checksum: 0,
        id: request.id,
        sequence: request.sequence,
    };

    let mut full_payload = Vec::with_capacity(8 + data.len());
    full_payload.extend_from_slice(reply.to_bytes());
    full_payload.extend_from_slice(data);

    let cs = IcmpPacket::calculate_checksum_raw(&full_payload);
    full_payload[2] = (cs >> 8) as u8;
    full_payload[3] = (cs & 0xFF) as u8;

    send(target_ip, 1, &full_payload);
}
