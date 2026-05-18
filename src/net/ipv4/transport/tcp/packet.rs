#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_res: u8,
    pub flags: u8,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
}

pub const PROTOCOL: u8 = 6;

pub mod flags {
    pub const FIN: u8 = 1 << 0; // 0x01
    pub const SYN: u8 = 1 << 1; // 0x02
    pub const RST: u8 = 1 << 2; // 0x04
    pub const PSH: u8 = 1 << 3; // 0x08
    pub const ACK: u8 = 1 << 4; // 0x10
    pub const URG: u8 = 1 << 5; // 0x20
}

pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    Closing,
    TimeWait,
    CloseWait,
    LastAck,
}

impl TcpHeader {
    pub fn new(src_port: u16, dst_port: u16, seq: u32, ack: u32, tcp_flags: u8) -> Self {
        Self {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            seq_num: seq.to_be(),
            ack_num: ack.to_be(),
            data_offset_res: 5 << 4,
            flags: tcp_flags,
            window_size: 1024u16.to_be(),
            checksum: 0,
            urgent_pointer: 0,
        }
    }

    pub fn calculate_checksum(
        &mut self,
        src_ip: &[u8; 4],
        dst_ip: &[u8; 4],
        payload: &[u8],
    ) -> u16 {
        self.checksum = 0;

        let mut sum: u32 = 0;

        sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
        sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;

        sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
        sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;

        sum += 6u32;

        let tcp_length = (core::mem::size_of::<Self>() + payload.len()) as u16;
        sum += tcp_length as u32;

        let header_bytes = self.as_bytes();
        for i in (0..header_bytes.len()).step_by(2) {
            let word = u16::from_be_bytes([header_bytes[i], header_bytes[i + 1]]);
            sum += word as u32;
        }

        for i in (0..(payload.len() & !1)).step_by(2) {
            let word = u16::from_be_bytes([payload[i], payload[i + 1]]);
            sum += word as u32;
        }

        if (payload.len() & 1) == 1 {
            let last_byte_word = u16::from_be_bytes([payload[payload.len() - 1], 0]);
            sum += last_byte_word as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        self.checksum = (!(sum as u16)).to_be();
        self.checksum
    }

    pub fn verify_checksum(&self, src_ip: &[u8; 4], dst_ip: &[u8; 4], payload: &[u8]) -> bool {
        let mut sum: u32 = 0;

        sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
        sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;

        sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
        sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;

        sum += PROTOCOL as u32;

        let tcp_length = (core::mem::size_of::<Self>() + payload.len()) as u16;
        sum += tcp_length as u32;

        let header_bytes = self.as_bytes();
        for i in (0..header_bytes.len()).step_by(2) {
            let word = u16::from_be_bytes([header_bytes[i], header_bytes[i + 1]]);
            sum += word as u32;
        }

        for i in (0..(payload.len() & !1)).step_by(2) {
            let word = u16::from_be_bytes([payload[i], payload[i + 1]]);
            sum += word as u32;
        }

        if (payload.len() & 1) == 1 {
            let last_byte_word = u16::from_be_bytes([payload[payload.len() - 1], 0]);
            sum += last_byte_word as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        (sum as u16) == 0xFFFF
    }
}

pub fn verify_segment_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_segment: &[u8]) -> bool {
    let mut sum: u32 = 0;

    sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
    sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;
    sum += PROTOCOL as u32;
    sum += tcp_segment.len() as u32;

    for i in (0..(tcp_segment.len() & !1)).step_by(2) {
        sum += u16::from_be_bytes([tcp_segment[i], tcp_segment[i + 1]]) as u32;
    }
    if (tcp_segment.len() & 1) == 1 {
        sum += u16::from_be_bytes([tcp_segment[tcp_segment.len() - 1], 0]) as u32;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    (sum as u16) == 0xFFFF
}

impl TcpHeader {

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        unsafe { core::ptr::read_unaligned(slice.as_ptr() as *const Self) }
    }
}
