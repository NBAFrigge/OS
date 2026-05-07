#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ip_Header {
    pub version_ihl: u8, // Version (4b) + IHL (4b) uniti in un byte
    pub tos: u8,
    pub total_length: u16,   // Big Endian
    pub id: u16,             // Big Endian
    pub flags_fragment: u16, // Flags (3b) + Offset (13b)
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16, // Big Endian
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
}

impl ip_Header {
    pub fn version(&self) -> u8 {
        self.version_ihl >> 4
    }

    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }

    pub fn flags(&self) -> u16 {
        self.flags_fragment >> 13
    }

    pub fn fragment(&self) -> u16 {
        self.flags_fragment & 0x1FFF
    }

    pub fn calculate_checksum(&mut self) {
        self.checksum = 0;
        let bytes = unsafe { core::slice::from_raw_parts((self as *const Self) as *const u16, 10) };

        let mut sum: u32 = 0;
        for &word in bytes {
            sum += u16::from_be(word) as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        self.checksum = (!(sum as u16)).to_be();
    }
}
