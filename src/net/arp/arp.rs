#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hw_addr_len: u8,
    pub proto_addr_len: u8,
    pub operation: u16,
    pub sender_hw_addr: [u8; 6],
    pub sender_proto_addr: [u8; 4],
    pub target_hw_addr: [u8; 6],
    pub target_proto_addr: [u8; 4],
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HardwareType {
    Ethernet = 1,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProtocolType {
    Ipv4 = 0x0800,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArpOperation {
    Request = 1,
    Reply = 2,
}

impl ArpPacket {
    pub fn new(
        operation: ArpOperation,
        sender_hw_addr: [u8; 6],
        sender_proto_addr: [u8; 4],
        target_hw_addr: [u8; 6],
        target_proto_addr: [u8; 4],
    ) -> Self {
        Self {
            hardware_type: (HardwareType::Ethernet as u16).to_be(),
            protocol_type: (ProtocolType::Ipv4 as u16).to_be(),
            hw_addr_len: 6,
            proto_addr_len: 4,
            operation: (operation as u16).to_be(),
            sender_hw_addr,
            sender_proto_addr,
            target_hw_addr,
            target_proto_addr,
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<ArpPacket>() {
            return None;
        }

        let packet = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const ArpPacket) };

        Some(Self {
            hardware_type: u16::from_be(packet.hardware_type),
            protocol_type: u16::from_be(packet.protocol_type),
            hw_addr_len: packet.hw_addr_len,
            proto_addr_len: packet.proto_addr_len,
            operation: u16::from_be(packet.operation),
            sender_hw_addr: packet.sender_hw_addr,
            sender_proto_addr: packet.sender_proto_addr,
            target_hw_addr: packet.target_hw_addr,
            target_proto_addr: packet.target_proto_addr,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                (self as *const ArpPacket) as *const u8,
                core::mem::size_of::<ArpPacket>(),
            )
        }
    }
}
