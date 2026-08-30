use alloc::vec::Vec;

pub struct UdpPacketData {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub payload: Vec<u8>,
}

impl UdpPacketData {
    pub fn new(src_ip: [u8; 4], src_port: u16, payload: Vec<u8>) -> Self {
        Self {
            src_ip,
            src_port,
            payload,
        }
    }
}

#[repr(C, packed)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn new(src_port: u16, dst_port: u16) -> Self {
        Self {
            src_port,
            dst_port,
            length: 8,
            checksum: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];

        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.length.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.checksum.to_be_bytes());

        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        }
    }

    pub fn calculate_checksum(
        &self,
        src_ip: &[u8; 4],
        dst_ip: &[u8; 4],
        payload: &[u8],
    ) -> u16 {
        let mut sum: u32 = 0;

        for i in (0..4).step_by(2) {
            sum += u16::from_be_bytes([src_ip[i], src_ip[i + 1]]) as u32;
            sum += u16::from_be_bytes([dst_ip[i], dst_ip[i + 1]]) as u32;
        }

        sum += 17u32;
        sum += self.length as u32;

        sum += self.src_port as u32;
        sum += self.dst_port as u32;
        sum += self.length as u32;

        for i in (0..payload.len() & !1).step_by(2) {
            sum += u16::from_be_bytes([payload[i], payload[i + 1]]) as u32;
        }

        if payload.len() % 2 != 0 {
            sum += (payload[payload.len() - 1] as u32) << 8;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        let result = !(sum as u16);
        let final_checksum = if result == 0 { 0xFFFF } else { result };

        final_checksum
    }

    pub fn serialize(
        &mut self,
        src_ip: &[u8; 4],
        dst_ip: &[u8; 4],
        payload: &[u8],
    ) -> [u8; 8] {
        self.length = (8 + payload.len()) as u16;
        self.checksum = 0;
        self.checksum = self.calculate_checksum(src_ip, dst_ip, payload);
        self.to_bytes()
    }
}
