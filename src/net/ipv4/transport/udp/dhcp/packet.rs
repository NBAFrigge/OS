use core::sync::atomic::AtomicU8;

use crate::net::{
    interface::NETWORK_INTERFACE,
    ipv4::transport::udp::{
        dhcp::packet::constants::{
            DHCP_MAGIC_COOKIE, FLAGS_BROADCAST, HLEN_ETHERNET, HTYPE_ETHERNET,
            OPT_END, OPT_PAD, OP_BOOTREQUEST,
        },
        packet,
    },
};

pub mod constants {
    pub const DHCP_CLIENT_PORT: u16 = 68;
    pub const DHCP_SERVER_PORT: u16 = 67;
    pub const DHCP_MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];

    pub const OP_BOOTREQUEST: u8 = 1;
    pub const OP_BOOTREPLY: u8 = 2;

    pub const HTYPE_ETHERNET: u8 = 1;
    pub const HLEN_ETHERNET: u8 = 6;

    pub const FLAGS_BROADCAST: u16 = 0x8000;
    pub const FLAGS_NONE: u16 = 0x0000;

    pub const DHCPDISCOVER: u8 = 1;
    pub const DHCPOFFER: u8 = 2;
    pub const DHCPREQUEST: u8 = 3;
    pub const DHCPDECLINE: u8 = 4;
    pub const DHCPACK: u8 = 5;
    pub const DHCPNAK: u8 = 6;
    pub const DHCPRELEASE: u8 = 7;
    pub const DHCPINFORM: u8 = 8;

    pub const OPT_PAD: u8 = 0;
    pub const OPT_SUBNET_MASK: u8 = 1;
    pub const OPT_TIME_OFFSET: u8 = 2;
    pub const OPT_ROUTER: u8 = 3;
    pub const OPT_TIME_SERVER: u8 = 4;
    pub const OPT_NAME_SERVER: u8 = 5;
    pub const OPT_DNS_SERVER: u8 = 6;
    pub const OPT_LOG_SERVER: u8 = 7;
    pub const OPT_HOSTNAME: u8 = 12;
    pub const OPT_DOMAIN_NAME: u8 = 15;
    pub const OPT_INTERFACE_MTU: u8 = 26;
    pub const OPT_BROADCAST_ADDRESS: u8 = 28;
    pub const OPT_REQUESTED_IP: u8 = 50;
    pub const OPT_LEASE_TIME: u8 = 51;
    pub const OPT_OPTION_OVERLOAD: u8 = 52;
    pub const OPT_MESSAGE_TYPE: u8 = 53;
    pub const OPT_SERVER_IDENTIFIER: u8 = 54;
    pub const OPT_PARAMETER_REQUEST_LIST: u8 = 55;
    pub const OPT_MESSAGE: u8 = 56;
    pub const OPT_MAX_DHCP_MESSAGE_SIZE: u8 = 57;
    pub const OPT_RENEWAL_TIME: u8 = 58;
    pub const OPT_REBINDING_TIME: u8 = 59;
    pub const OPT_VENDOR_CLASS_ID: u8 = 60;
    pub const OPT_CLIENT_ID: u8 = 61;
    pub const OPT_TFTP_SERVER_NAME: u8 = 66;
    pub const OPT_BOOTFILE_NAME: u8 = 67;
    pub const OPT_END: u8 = 255;

    pub const MIN_DHCP_PACKET_SIZE: usize = 300;
    pub const MAX_DHCP_PACKET_SIZE: usize = 576;
}

#[repr(C, packed)]
pub struct DhcpHeader {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub flags: u16,
    pub ciaddr: [u8; 4],
    pub yiaddr: [u8; 4],
    pub siaddr: [u8; 4],
    pub giaddr: [u8; 4],
    pub chaddr: [u8; 16],
    pub sname: [u8; 64],
    pub file: [u8; 128],
}

impl DhcpHeader {
    fn new_base(xid: u32) -> Self {
        let hw_addr = NETWORK_INTERFACE.lock().hw_addr.expect("No HW Addr");
        let mut chaddr = [0u8; 16];
        chaddr[..6].copy_from_slice(&hw_addr);

        DhcpHeader {
            op: OP_BOOTREQUEST,
            htype: HTYPE_ETHERNET,
            hlen: HLEN_ETHERNET,
            hops: 0,
            xid: xid.to_be(),
            secs: 0u16.to_be(),
            flags: FLAGS_BROADCAST.to_be(),
            ciaddr: [0; 4],
            yiaddr: [0; 4],
            siaddr: [0; 4],
            giaddr: [0; 4],
            chaddr,
            sname: [0; 64],
            file: [0; 128],
        }
    }

    fn new_discover(xid: u32) -> Self {
        Self::new_base(xid)
    }

    fn new_request(xid: u32) -> Self {
        Self::new_base(xid)
    }
}

#[repr(C, packed)]
pub struct DhcpPacket {
    pub header: DhcpHeader,
    pub magic_cookie: [u8; 4],
    pub options: [u8; 308],
}

impl DhcpPacket {
    pub fn new_discover(xid: u32) -> Self {
        let mut options = [0u8; 308];
        let mut cursor = 0;

        let mut add_opt = |tag: u8, data: &[u8]| {
            options[cursor] = tag;
            options[cursor + 1] = data.len() as u8;
            options[cursor + 2..cursor + 2 + data.len()].copy_from_slice(data);
            cursor += 2 + data.len();
        };

        add_opt(constants::OPT_MESSAGE_TYPE, &[constants::DHCPDISCOVER]);
        add_opt(constants::OPT_PARAMETER_REQUEST_LIST, &[1, 3, 6]);
        options[cursor] = constants::OPT_END;

        DhcpPacket {
            header: DhcpHeader::new_discover(xid),
            magic_cookie: constants::DHCP_MAGIC_COOKIE,
            options,
        }
    }

    pub fn new_request(
        xid: u32,
        requested_ip: &[u8; 4],
        server_ip: &[u8; 4],
    ) -> Self {
        let mut options = [0u8; 308];
        let mut cursor = 0;

        let mut add_opt = |tag: u8, data: &[u8]| {
            options[cursor] = tag;
            options[cursor + 1] = data.len() as u8;
            options[cursor + 2..cursor + 2 + data.len()].copy_from_slice(data);
            cursor += 2 + data.len();
        };

        add_opt(constants::OPT_MESSAGE_TYPE, &[constants::DHCPREQUEST]);
        add_opt(constants::OPT_REQUESTED_IP, requested_ip);
        add_opt(constants::OPT_SERVER_IDENTIFIER, server_ip);
        add_opt(constants::OPT_PARAMETER_REQUEST_LIST, &[1, 3, 6]);
        options[cursor] = constants::OPT_END;

        DhcpPacket {
            header: DhcpHeader::new_request(xid),
            magic_cookie: constants::DHCP_MAGIC_COOKIE,
            options,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<&Self> {
        if data.len() < 240 {
            return None;
        }

        let packet = unsafe { &*(data.as_ptr() as *const Self) };
        if packet.magic_cookie != DHCP_MAGIC_COOKIE {
            return None;
        }

        Some(packet)
    }

    pub fn get_option(&self, target_tag: u8) -> Option<&[u8]> {
        let mut cursor = 0;

        while cursor < self.options.len() {
            let tag = self.options[cursor];

            if tag == OPT_END {
                break;
            }
            if tag == OPT_PAD {
                cursor += 1;
                continue;
            }

            if cursor + 1 >= self.options.len() {
                break;
            }
            let len = self.options[cursor + 1] as usize;

            if cursor + 2 + len > self.options.len() {
                break;
            }

            if tag == target_tag {
                return Some(&self.options[cursor + 2..cursor + 2 + len]);
            }

            cursor += 2 + len;
        }
        None
    }
}
