use alloc::vec::Vec;
use crate::kwarn;

pub struct EthernetPacket<'a> {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: u16,
    pub payload: &'a [u8],
}

pub fn parse(frame: &[u8]) -> Option<EthernetPacket> {
    if frame.len() < 14 {
        kwarn!("Ethernet: frame too short ({} bytes)", frame.len());
        return None;
    }

    let dst_mac = frame[0..6].try_into().ok()?;
    let src_mac = frame[6..12].try_into().ok()?;
    let ether_type = ((frame[12] as u16) << 8) | frame[13] as u16;
    let payload = &frame[14..];

    Some(EthernetPacket {
        dst_mac,
        src_mac,
        ether_type,
        payload,
    })
}

pub fn build(dst_mac: [u8; 6], src_mac: [u8; 6], ether_type: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac);
    frame.extend_from_slice(&src_mac);
    frame.push((ether_type >> 8) as u8);
    frame.push((ether_type & 0xFF) as u8);
    frame.extend_from_slice(payload);
    frame
}
