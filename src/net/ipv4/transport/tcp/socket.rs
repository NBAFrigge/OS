use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    sync::Arc,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    crypto::random::{generate_u16, generate_u32},
    kdebug, kinfo, ktrace, kwarn,
    net::{
        interface::NETWORK_INTERFACE,
        ipv4::{
            self,
            transport::tcp::{
                self,
                packet::{self, verify_segment_checksum, TcpHeader, PROTOCOL},
            },
        },
    },
};

use packet::flags;

const SYN_RETRANSMIT_INTERVAL: u32 = 500;
const DATA_RETRANSMIT_INTERVAL: u32 = 1000;
const MSS: usize = 536;

#[derive(PartialEq, Eq, Debug)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpError {
    NotReady,
    WrongState,
    BufferFull,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, PartialOrd, Ord, Debug)]
pub struct TcpTuple {
    pub local_port: u16,
    pub remote_ip: [u8; 4],
    pub remote_port: u16,
}

pub struct TcpSocket {
    pub tuple: TcpTuple,
    pub state: TcpState,
    pub send_unacked: u32,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub rx_queue: VecDeque<u8>,
    pub tx_queue: VecDeque<u8>,
    pub retransmit_ticks: u32,
    pub max_queue_size: usize,
}

impl TcpSocket {
    fn new(remote_ip: &[u8; 4], remote_port: u16) -> Self {
        let tuple = TcpTuple {
            local_port: 49152 + generate_u16() % 16384,
            remote_ip: *remote_ip,
            remote_port,
        };
        Self {
            tuple,
            state: TcpState::Closed,
            send_unacked: 0,
            local_seq: 0,
            remote_seq: 0,
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            retransmit_ticks: 0,
            max_queue_size: 65536,
        }
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> usize {
        let mut n = 0;
        while n < buffer.len() {
            match self.rx_queue.pop_front() {
                Some(byte) => {
                    buffer[n] = byte;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }

    pub fn write(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let space = self.max_queue_size.saturating_sub(self.tx_queue.len());
        for &byte in &data[..data.len().min(space)] {
            self.tx_queue.push_back(byte);
        }
    }

    fn sent_unacked_len(&self) -> usize {
        self.local_seq.wrapping_sub(self.send_unacked) as usize
    }

    pub fn close(&mut self) -> Result<(), TcpError> {
        let next_state = match self.state {
            TcpState::Established => TcpState::FinWait1,
            TcpState::CloseWait => TcpState::LastAck,
            _ => return Err(TcpError::WrongState),
        };

        let src_ip = match NETWORK_INTERFACE.lock().ip_addr {
            Some(ip) if ip != [0, 0, 0, 0] => ip,
            _ => return Err(TcpError::NotReady),
        };

        let mut fin_header = TcpHeader::new(
            self.tuple.local_port,
            self.tuple.remote_port,
            self.local_seq,
            self.remote_seq,
            flags::FIN | flags::ACK,
        );

        fin_header.calculate_checksum(&src_ip, &self.tuple.remote_ip, &[]);
        let fin_bytes = fin_header.as_bytes();

        ipv4::protocol::send(
            self.tuple.remote_ip,
            ipv4::transport::tcp::packet::PROTOCOL,
            fin_bytes,
        );

        self.local_seq = self.local_seq.wrapping_add(1);
        self.state = next_state;

        Ok(())
    }
}

fn send_segment(
    src_ip: &[u8; 4],
    remote_ip: [u8; 4],
    local_port: u16,
    remote_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    payload: &[u8],
) {
    let mut header = TcpHeader::new(local_port, remote_port, seq, ack, flags);
    header.calculate_checksum(src_ip, &remote_ip, payload);

    let mut packet = alloc::vec::Vec::with_capacity(20 + payload.len());
    packet.extend_from_slice(header.as_bytes());
    packet.extend_from_slice(payload);

    ipv4::protocol::send(remote_ip, PROTOCOL, &packet);
}

pub fn connect(remote_ip: &[u8; 4], remote_port: u16) -> u16 {
    let mut socket = TcpSocket::new(remote_ip, remote_port);
    let isn = generate_u32();
    socket.send_unacked = isn;
    socket.local_seq = isn.wrapping_add(1);

    let src_ip = if let Some(ip) = NETWORK_INTERFACE.lock().ip_addr {
        ip
    } else {
        kwarn!("TCP: Cannot connect, network interface has no IP");
        return 0;
    };

    let mut syn = packet::TcpHeader::new(socket.tuple.local_port, remote_port, isn, 0, flags::SYN);
    syn.calculate_checksum(&src_ip, remote_ip, &[]);
    ipv4::protocol::send(*remote_ip, packet::PROTOCOL, syn.as_bytes());
    socket.state = TcpState::SynSent;

    let local_port = socket.tuple.local_port;
    TCP_SOCKET_MANAGER
        .lock()
        .insert(socket.tuple, Arc::new(Mutex::new(socket)));

    local_port
}

pub fn handle_tcp_packet(src_ip: &[u8; 4], raw_packet: &[u8]) {
    if raw_packet.len() < 20 {
        ktrace!("TCP: packet too short");
        return;
    }

    let dst_ip = match NETWORK_INTERFACE.lock().ip_addr {
        Some(ip) => ip,
        None => return,
    };

    if !verify_segment_checksum(src_ip, &dst_ip, raw_packet) {
        kwarn!("TCP: checksum mismatch, dropping");
        return;
    }

    let header = TcpHeader::from_bytes(&raw_packet[0..20]);
    let src_port = u16::from_be(header.src_port);
    let dst_port = u16::from_be(header.dst_port);
    let seq_num = u32::from_be(header.seq_num);
    let ack_num = u32::from_be(header.ack_num);
    let tcp_header_len = ((header.data_offset_res >> 4) as usize) * 4;

    if raw_packet.len() < tcp_header_len {
        kwarn!("TCP: packet smaller than declared header length");
        return;
    }

    let payload = &raw_packet[tcp_header_len..];

    let key = TcpTuple {
        local_port: dst_port,
        remote_ip: *src_ip,
        remote_port: src_port,
    };

    let manager = TCP_SOCKET_MANAGER.lock();
    let Some(socket_handle) = manager.get(&key) else {
        ktrace!("TCP: no socket for tuple, dropping");
        return;
    };
    let mut socket = socket_handle.lock();

    match socket.state {
        TcpState::SynSent => {
            if (header.flags & (flags::SYN | flags::ACK)) != (flags::SYN | flags::ACK) {
                kwarn!("TCP: expected SYN-ACK in SynSent, dropping");
                return;
            }

            socket.remote_seq = seq_num.wrapping_add(1);
            socket.send_unacked = ack_num;
            socket.local_seq = ack_num;

            let mut ack =
                TcpHeader::new(dst_port, src_port, ack_num, socket.remote_seq, flags::ACK);
            ack.calculate_checksum(&dst_ip, src_ip, &[]);
            ipv4::protocol::send(*src_ip, PROTOCOL, ack.as_bytes());

            kdebug!(
                "TCP: connection established port {} ← {}.{}.{}.{}:{}",
                dst_port,
                src_ip[0],
                src_ip[1],
                src_ip[2],
                src_ip[3],
                src_port
            );
            socket.state = TcpState::Established;
        }

        TcpState::Established => {
            if (header.flags & flags::ACK) != 0 {
                let bytes_acked = ack_num.wrapping_sub(socket.send_unacked) as usize;
                if bytes_acked > 0 && bytes_acked <= socket.sent_unacked_len() {
                    for _ in 0..bytes_acked {
                        socket.tx_queue.pop_front();
                    }
                    socket.send_unacked = ack_num;
                    socket.retransmit_ticks = 0;
                }
            }

            let mut invia_ack_dati = false;
            if !payload.is_empty() {
                socket.rx_queue.extend(payload.iter().copied());
                socket.remote_seq = socket.remote_seq.wrapping_add(payload.len() as u32);
                invia_ack_dati = true;
            }

            if (header.flags & flags::FIN) != 0 {
                socket.remote_seq = socket.remote_seq.wrapping_add(1);
                socket.state = TcpState::CloseWait;
                kdebug!("TCP: Received FIN from server. Moving to CloseWait.");

                let mut ack_packet = TcpHeader::new(
                    socket.tuple.local_port,
                    socket.tuple.remote_port,
                    socket.local_seq,
                    socket.remote_seq,
                    flags::ACK,
                );
                ack_packet.calculate_checksum(&dst_ip, &socket.tuple.remote_ip, &[]);
                ipv4::protocol::send(socket.tuple.remote_ip, PROTOCOL, ack_packet.as_bytes());
            } else if invia_ack_dati {
                let mut ack = tcp::packet::TcpHeader::new(
                    key.local_port,
                    key.remote_port,
                    socket.local_seq,
                    socket.remote_seq,
                    flags::ACK,
                );
                ack.calculate_checksum(src_ip, &dst_ip, &[]);
                ipv4::protocol::send(key.remote_ip, PROTOCOL, ack.as_bytes());
            }
        }

        TcpState::FinWait1 => {
            if (header.flags & flags::ACK) != 0 && ack_num == socket.local_seq {
                socket.state = TcpState::FinWait2;
            }
        }

        TcpState::FinWait2 => {
            if (header.flags & flags::FIN) != 0 {
                socket.remote_seq = socket.remote_seq.wrapping_add(1);

                let mut ack = TcpHeader::new(
                    dst_port,
                    src_port,
                    socket.local_seq,
                    socket.remote_seq,
                    flags::ACK,
                );
                ack.calculate_checksum(&dst_ip, src_ip, &[]);
                ipv4::protocol::send(*src_ip, PROTOCOL, ack.as_bytes());

                socket.state = TcpState::Closed;
            }
        }

        TcpState::LastAck => {
            let bytes_acked = ack_num.wrapping_sub(socket.send_unacked) as usize;
            if (header.flags & flags::ACK) != 0
                && bytes_acked > 0
                && bytes_acked <= socket.sent_unacked_len()
            {
                for _ in 0..bytes_acked {
                    socket.tx_queue.pop_front();
                }
                socket.send_unacked = ack_num;
                socket.retransmit_ticks = 0;
            }
        }

        _ => kwarn!("TCP: packet in unhandled state"),
    }
}

pub fn tcp_tick() {
    let src_ip = match NETWORK_INTERFACE.lock().ip_addr {
        Some(ip) if ip != [0, 0, 0, 0] => ip,
        _ => return,
    };

    let mut to_remove: alloc::vec::Vec<TcpTuple> = alloc::vec::Vec::new();

    {
        let manager = TCP_SOCKET_MANAGER.lock();
        for (tuple, socket_handle) in manager.iter() {
            let mut socket = socket_handle.lock();

            let closing = socket.state == TcpState::FinWait1 || socket.state == TcpState::LastAck;
            if socket.sent_unacked_len() > 0 || closing {
                socket.retransmit_ticks += 1;
            } else {
                socket.retransmit_ticks = 0;
            }

            match socket.state {
                TcpState::SynSent => {
                    socket.retransmit_ticks += 1;
                    if socket.retransmit_ticks >= SYN_RETRANSMIT_INTERVAL {
                        socket.retransmit_ticks = 0;
                        kdebug!(
                            "TCP: retransmitting SYN to port {}",
                            socket.tuple.remote_port
                        );
                        send_segment(
                            &src_ip,
                            socket.tuple.remote_ip,
                            socket.tuple.local_port,
                            socket.tuple.remote_port,
                            socket.send_unacked,
                            0,
                            flags::SYN,
                            &[],
                        );
                    }
                }

                TcpState::Established => {
                    let sent = socket.sent_unacked_len();
                    let unsent = socket.tx_queue.len().saturating_sub(sent);

                    if unsent > 0 {
                        let to_send = unsent.min(MSS);
                        let segment: alloc::vec::Vec<u8> = socket
                            .tx_queue
                            .iter()
                            .skip(sent)
                            .take(to_send)
                            .copied()
                            .collect();

                        send_segment(
                            &src_ip,
                            socket.tuple.remote_ip,
                            socket.tuple.local_port,
                            socket.tuple.remote_port,
                            socket.local_seq,
                            socket.remote_seq,
                            flags::ACK | flags::PSH,
                            &segment,
                        );
                        socket.local_seq = socket.local_seq.wrapping_add(to_send as u32);
                    }

                    if socket.retransmit_ticks >= DATA_RETRANSMIT_INTERVAL {
                        socket.retransmit_ticks = 0;
                        let to_send = sent.min(MSS);
                        let segment: alloc::vec::Vec<u8> =
                            socket.tx_queue.iter().take(to_send).copied().collect();

                        kdebug!("TCP: retransmitting {} bytes", to_send);
                        send_segment(
                            &src_ip,
                            socket.tuple.remote_ip,
                            socket.tuple.local_port,
                            socket.tuple.remote_port,
                            socket.send_unacked,
                            socket.remote_seq,
                            flags::ACK | flags::PSH,
                            &segment,
                        );
                    }
                }

                TcpState::FinWait1 | TcpState::LastAck => {
                    if socket.retransmit_ticks >= DATA_RETRANSMIT_INTERVAL {
                        socket.retransmit_ticks = 0;
                        kdebug!("TCP: retransmitting FIN in state {:?}", socket.state);

                        let fin_seq = socket.local_seq.wrapping_sub(1);

                        send_segment(
                            &src_ip,
                            socket.tuple.remote_ip,
                            socket.tuple.local_port,
                            socket.tuple.remote_port,
                            fin_seq,
                            socket.remote_seq,
                            flags::FIN | flags::ACK,
                            &[],
                        );
                    }
                }

                _ => {}
            }

            if socket.state == TcpState::Closed {
                to_remove.push(*tuple);
            }
        }
    }

    if !to_remove.is_empty() {
        let mut manager = TCP_SOCKET_MANAGER.lock();
        for tuple in &to_remove {
            manager.remove(tuple);
            kinfo!(
                "TCP: Socket removed {} -> {}.{}.{}.{}:{} from the manager",
                tuple.local_port,
                tuple.remote_ip[0],
                tuple.remote_ip[1],
                tuple.remote_ip[2],
                tuple.remote_ip[3],
                tuple.remote_port
            );
        }
    }
}
pub type SocketHandle = Arc<Mutex<TcpSocket>>;

lazy_static! {
    pub static ref TCP_SOCKET_MANAGER: Mutex<BTreeMap<TcpTuple, Arc<Mutex<TcpSocket>>>> =
        Mutex::new(BTreeMap::new());
}
