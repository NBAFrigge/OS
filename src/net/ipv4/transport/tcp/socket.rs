use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    sync::Arc,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    crypto::random::{generate_u16, generate_u32},
    kdebug, ktrace, kwarn,
    net::{
        interface::NETWORK_INTERFACE,
        ipv4::{
            self,
            transport::tcp::packet::{
                self, verify_segment_checksum, TcpHeader, TcpState, PROTOCOL,
            },
        },
    },
};

use packet::flags;

const SYN_RETRANSMIT_INTERVAL: u32 = 500;

#[derive(Hash, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
pub struct TcpTuple {
    pub local_port: u16,
    pub remote_ip: [u8; 4],
    pub remote_port: u16,
}

pub struct TcpSocket {
    pub tuple: TcpTuple,
    pub state: TcpState,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub rx_queue: VecDeque<u8>,
    pub tx_queue: VecDeque<u8>,
    pub retransmit_ticks: u32,
}

impl TcpSocket {
    fn new(remote_ip: &[u8; 4], remote_port: u16) -> Self {
        let tcp_record = TcpTuple {
            local_port: 49152 + generate_u16() % 16384,
            remote_ip: *remote_ip,
            remote_port,
        };
        Self {
            tuple: tcp_record,
            state: TcpState::Closed,
            local_seq: 0,
            remote_seq: 0,
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            retransmit_ticks: 0,
        }
    }
}

pub fn connect(remote_ip: &[u8; 4], remote_port: u16) {
    let mut socket = TcpSocket::new(remote_ip, remote_port);
    let isn = generate_u32();
    socket.local_seq = isn;
    let mut syn_header =
        packet::TcpHeader::new(socket.tuple.local_port, remote_port, isn, 0, flags::SYN);

    let src_ip = if let Some(ip) = NETWORK_INTERFACE.lock().ip_addr {
        ip
    } else {
        kwarn!("TCP: Cannot connect, network interface has no IP");
        return;
    };

    syn_header.calculate_checksum(&src_ip, remote_ip, &[]);
    let syn_bytes = syn_header.as_bytes();
    ipv4::protocol::send(*remote_ip, packet::PROTOCOL, syn_bytes);
    socket.state = TcpState::SynSent;
    TCP_SOCKET_MANAGER
        .lock()
        .insert(socket.tuple, Arc::new(Mutex::new(socket)));
}

pub fn handle_tcp_packet(src_ip: &[u8; 4], raw_packet: &[u8]) {
    if raw_packet.len() < 20 {
        ktrace!("Tcp packet too short");
        return;
    }

    let header_bytes = &raw_packet[0..20];
    let header = TcpHeader::from_bytes(header_bytes);
    let dst_ip = if let Some(ip) = NETWORK_INTERFACE.lock().ip_addr {
        ip
    } else {
        kwarn!("TCP: Cannot connect, network interface has no IP");
        return;
    };

    let src_port = u16::from_be(header.src_port);
    let dst_port = u16::from_be(header.dst_port);
    let seq_num = u32::from_be(header.seq_num);
    let ack_num = u32::from_be(header.ack_num);

    let tcp_header_len = ((header.data_offset_res >> 4) as usize) * 4;
    if raw_packet.len() < tcp_header_len {
        kwarn!("TCP: Packet smaller than its declared header length");
        return;
    }

    let payload = &raw_packet[tcp_header_len..];
    let payload_len = payload.len();

    if !verify_segment_checksum(src_ip, &dst_ip, raw_packet) {
        kwarn!("TCO: checksum mismatch on port {}, dropping", dst_port);
        return;
    }

    let key = TcpTuple {
        local_port: dst_port,
        remote_ip: *src_ip,
        remote_port: src_port,
    };

    let manager = TCP_SOCKET_MANAGER.lock();
    if let Some(socket_handle) = manager.get(&key) {
        let mut socket = socket_handle.lock();

        match socket.state {
            TcpState::SynSent => {
                let is_syn = (header.flags & flags::SYN) != 0;
                let is_ack = (header.flags & flags::ACK) != 0;

                if is_syn && is_ack {
                    kdebug!("TCP: Received SYN-ACK from server on port {}", dst_port);
                    socket.remote_seq = seq_num;
                    socket.local_seq = ack_num;

                    let mut ack_packet =
                        TcpHeader::new(dst_port, src_port, ack_num, seq_num + 1, flags::ACK);

                    ack_packet.calculate_checksum(&dst_ip, src_ip, &[]);
                    let ack_packet_bytes = ack_packet.as_bytes();
                    ipv4::protocol::send(*src_ip, PROTOCOL, ack_packet_bytes);

                    kdebug!(
                        "TCP: connetcion established on port {} from server {}.{}.{}.{}:{}",
                        dst_port,
                        key.remote_ip[0],
                        key.remote_ip[1],
                        key.remote_ip[2],
                        key.remote_ip[3],
                        key.remote_port
                    );
                    socket.state = TcpState::Established;
                } else {
                    kwarn!("TCP: Expected SYN-ACK in SynSent state, dropping packet");
                }
            }
            TcpState::Established => {
                if payload_len > 0 {
                    kdebug!(
                        "TCP: Received {} bytes of payload in Established state",
                        payload_len
                    );

                    for &byte in payload {
                        socket.rx_queue.push_back(byte);
                    }

                    socket.remote_seq += payload_len as u32;

                    // TODO: Send ack packet
                }
            }
            _ => {
                kwarn!("TCP: Packet received in unhandled state");
            }
        }
    } else {
        ktrace!("TCP: No socket found for tuple, dropping packet");
        // TODO: RST packet handling
    }
}

pub fn tcp_tick() {
    let src_ip = match NETWORK_INTERFACE.lock().ip_addr {
        Some(ip) if ip != [0, 0, 0, 0] => ip,
        _ => return,
    };

    let manager = TCP_SOCKET_MANAGER.lock();
    for socket_handle in manager.values() {
        let mut socket = socket_handle.lock();
        if matches!(socket.state, TcpState::SynSent) {
            socket.retransmit_ticks += 1;
            if socket.retransmit_ticks >= SYN_RETRANSMIT_INTERVAL {
                socket.retransmit_ticks = 0;
                let remote_ip = socket.tuple.remote_ip;
                let isn = socket.local_seq;
                let mut syn_header = TcpHeader::new(
                    socket.tuple.local_port,
                    socket.tuple.remote_port,
                    isn,
                    0,
                    flags::SYN,
                );
                syn_header.calculate_checksum(&src_ip, &remote_ip, &[]);
                let syn_bytes = syn_header.as_bytes();
                kdebug!(
                    "TCP: retransmitting SYN to port {}",
                    socket.tuple.remote_port
                );
                ipv4::protocol::send(remote_ip, PROTOCOL, syn_bytes);
            }
        }
    }
}

pub type SocketHandle = Arc<Mutex<TcpSocket>>;

lazy_static! {
    pub static ref TCP_SOCKET_MANAGER: Mutex<BTreeMap<TcpTuple, Arc<Mutex<TcpSocket>>>> =
        Mutex::new(BTreeMap::new());
}
