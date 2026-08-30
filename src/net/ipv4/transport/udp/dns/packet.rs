pub mod constants {
    pub const TYPE_A: u16 = 1;
    pub const TYPE_NS: u16 = 2;
    pub const TYPE_CNAME: u16 = 5;
    pub const TYPE_MX: u16 = 15;
    pub const TYPE_TXT: u16 = 16;
    pub const TYPE_AAAA: u16 = 28;

    pub const CLASS_IN: u16 = 1;

    pub const FLAG_QUERY: u16 = 0x0000;
    pub const FLAG_RESPONSE: u16 = 0x8000;

    pub const FLAG_RD: u16 = 0x0100;
    pub const FLAG_RA: u16 = 0x0080;

    pub const RCODE_NO_ERROR: u16 = 0;
    pub const RCODE_FORMAT_ERROR: u16 = 1;
    pub const RCODE_SERVER_FAILURE: u16 = 2;
    pub const RCODE_NAME_ERROR: u16 = 3;
    pub const RCODE_NOT_IMPLEMENTED: u16 = 4;
    pub const RCODE_REFUSED: u16 = 5;

    pub const TYPE_POINTER: u8 = 0xC0;
}

#[repr(C, packed)]
pub struct DnsHeader {
    pub id: u16,
    pub flags: u16,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

impl DnsHeader {
    pub fn new(id: u16, flags: u16) -> Self {
        Self {
            id: id.to_be(),
            flags: flags.to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<Self>() {
            return None;
        }
        Some(unsafe { core::ptr::read(data.as_ptr() as *const Self) })
    }
}

#[repr(C, packed)]
pub struct DnsQuestionFooter {
    pub qtype: u16,
    pub qclass: u16,
}

pub struct DnsQuestion<'a> {
    pub name: &'a [u8],
    pub footer: DnsQuestionFooter,
}

impl<'a> DnsQuestion<'a> {
    pub fn new(name: &'a [u8], qtype: u16, qclass: u16) -> Self {
        Self {
            name,
            footer: DnsQuestionFooter {
                qtype: qtype.to_be(),
                qclass: qclass.to_be(),
            },
        }
    }

    pub fn write_to_buffer(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        for part in self.name.split(|&b| b == b'.') {
            if part.is_empty() {
                continue;
            }
            buffer[pos] = part.len() as u8;
            pos += 1;
            buffer[pos..pos + part.len()].copy_from_slice(part);
            pos += part.len();
        }
        buffer[pos] = 0;
        pos += 1;

        let footer_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.footer as *const _ as *const u8,
                4,
            )
        };
        buffer[pos..pos + 4].copy_from_slice(footer_bytes);
        pos + 4
    }
}

#[repr(C, packed)]
pub struct DnsResourceRecordHeader {
    pub rtype: u16,
    pub rclass: u16,
    pub ttl: u32,
    pub rdlength: u16,
}

pub struct DnsResourceRecord<'a> {
    pub name: &'a [u8],
    pub header: DnsResourceRecordHeader,
    pub data: &'a [u8],
}

impl<'a> DnsResourceRecord<'a> {
    pub fn parse(packet: &'a [u8], mut pos: usize) -> Option<(Self, usize)> {
        let start_name = pos;
        while pos < packet.len() {
            let b = packet[pos];
            if b & 0xC0 == 0xC0 {
                pos += 2;
                break;
            } else if b == 0 {
                pos += 1;
                break;
            } else {
                pos += (b as usize) + 1;
            }
        }
        let name_slice = &packet[start_name..pos];

        if pos + 10 > packet.len() {
            return None;
        }
        let header = unsafe {
            core::ptr::read(
                packet.as_ptr().add(pos) as *const DnsResourceRecordHeader
            )
        };
        pos += 10;

        let rdlength = u16::from_be(header.rdlength) as usize;
        if pos + rdlength > packet.len() {
            return None;
        }
        let data = &packet[pos..pos + rdlength];
        pos += rdlength;

        Some((
            Self {
                name: name_slice,
                header,
                data,
            },
            pos,
        ))
    }
}
