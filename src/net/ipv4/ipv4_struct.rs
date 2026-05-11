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
        let ptr = self as *const Self as *const u16;
        let mut sum: u32 = 0;

        for i in 0..10 {
            sum += unsafe { ptr.add(i).read_unaligned() }.to_be() as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        self.checksum = (!(sum as u16)).to_be();
    }

    pub fn serialize(&self, payload: &[u8], target_buffer: &mut [u8]) -> usize {
        let header_size = core::mem::size_of::<Self>(); // Sempre 20
        let total_size = header_size + payload.len();

        let header_ptr = self as *const Self as *const u8;
        let header_slice = unsafe { core::slice::from_raw_parts(header_ptr, header_size) };

        target_buffer[0..header_size].copy_from_slice(header_slice);

        target_buffer[header_size..total_size].copy_from_slice(payload);

        total_size
    }
}
