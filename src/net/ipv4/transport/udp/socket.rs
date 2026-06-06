use alloc::collections::btree_map::BTreeMap;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::net::interface::NETWORK_INTERFACE;
use crate::net::ipv4;
use crate::net::ipv4::transport::udp::packet::{UdpHeader, UdpPacketData};
use crate::{kdebug, ktrace, kwarn};

const PROTOCOL: u8 = 17;

pub struct UdpSocket {
    pub local_port: u16,
    pub rx_queue: VecDeque<UdpPacketData>,
    pub max_queue_size: usize,
}

impl UdpSocket {
    fn new(port: u16) -> Self {
        Self {
            local_port: port,
            rx_queue: VecDeque::new(),
            max_queue_size: 128,
        }
    }

    pub fn bind(port: u16) -> Result<SocketHandle, &'static str> {
        let mut manager = UDP_SOCKET_MANAGER.lock();
        if manager.contains_key(&port) {
            return Err("Port already in use");
        }
        let socket = Arc::new(Mutex::new(UdpSocket::new(port)));
        manager.insert(port, Arc::clone(&socket));
        Ok(socket)
    }

    pub fn receive(&mut self) -> Option<UdpPacketData> {
        if self.rx_queue.is_empty() {
            return None;
        }

        self.rx_queue.pop_front()
    }

    pub fn send_to(&mut self, dst_ip: [u8; 4], dst_port: u16, data: &[u8]) {
        let src_ip = NETWORK_INTERFACE.lock().ip_addr.expect("IP not set");

        let mut packet_header = UdpHeader::new(self.local_port, dst_port);

        let serialized_header = packet_header.serialize(&src_ip, &dst_ip, data);

        let mut udp_datagram = Vec::with_capacity(8 + data.len());
        udp_datagram.extend_from_slice(&serialized_header);
        udp_datagram.extend_from_slice(data);

        ipv4::protocol::send(dst_ip, 17, &udp_datagram);
    }
}

pub type SocketHandle = Arc<Mutex<UdpSocket>>;

pub fn create_socket(port: u16) -> Arc<Mutex<UdpSocket>> {
    Arc::new(Mutex::new(UdpSocket::new(port)))
}

pub fn handle_udp_packet(src_ip: &[u8; 4], dst_ip: &[u8; 4], raw_packet: &[u8]) {
    if raw_packet.len() < 8 {
        ktrace!("Udp packet too short");
        return;
    }
    let header_bytes = &raw_packet[0..8];
    let header = UdpHeader::from_bytes(header_bytes);
    let payload_len = (header.length as usize)
        .saturating_sub(8)
        .min(raw_packet.len() - 8);
    let payload = &raw_packet[8..8 + payload_len];

    let checksum = header.checksum;
    let dst_port = header.dst_port;
    if checksum != 0 && checksum != header.calculate_checksum(src_ip, dst_ip, payload) {
        kwarn!("UDP: checksum mismatch on port {}, dropping", dst_port);
        return;
    }

    let src_port = header.src_port;
    let packet_data = UdpPacketData::new(*src_ip, src_port, payload.to_vec());
    let manager = UDP_SOCKET_MANAGER.lock();

    if let Some(socket_handle) = manager.get(&dst_port) {
        let mut socket = socket_handle.lock();

        if socket.rx_queue.len() < socket.max_queue_size {
            kdebug!("UDP: received {} bytes on port {}", payload.len(), dst_port);
            socket.rx_queue.push_back(packet_data);
        } else {
            kwarn!("UDP: rx queue full on port {}, dropping", dst_port);
        }
    }
}

lazy_static! {
    pub static ref UDP_SOCKET_MANAGER: Mutex<BTreeMap<u16, SocketHandle>> =
        Mutex::new(BTreeMap::new());
}
