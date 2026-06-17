use core::sync::atomic::{AtomicU16, Ordering};

use alloc::{string::ToString, vec::Vec};

use crate::{
    crypto::random::generate_u16,
    idt::interrupt::TICKS,
    kdebug, kerror,
    net::{
        interface::NETWORK_INTERFACE,
        ipv4::transport::udp::{
            self,
            dns::{
                cache::{get_from_cache, DnsCacheEntry, DNS_CACHE},
                packet::{
                    constants::{CLASS_IN, FLAG_RD, TYPE_A},
                    DnsHeader, DnsQuestionFooter, DnsResourceRecord,
                },
            },
            socket::UDP_SOCKET_MANAGER,
        },
    },
    task::task::sleep,
};

static DNS_PORT: AtomicU16 = AtomicU16::new(0);

fn get_dns_port() -> u16 {
    let port = DNS_PORT.load(Ordering::Relaxed);
    if port != 0 {
        return port;
    }
    let new_port = 49152 + (generate_u16() % 16384);
    match DNS_PORT.compare_exchange(0, new_port, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => new_port,
        Err(existing) => existing,
    }
}

pub struct DnsResolver;

impl DnsResolver {
    pub fn get(&self, domain: &str) -> Option<[u8; 4]> {
        if let Some(ip) = get_from_cache(domain) {
            kdebug!("DNS: cache hit for {}", domain);
            return Some(ip);
        }

        {
            let port = get_dns_port();
            let mut manager = UDP_SOCKET_MANAGER.lock();
            if manager.get(&port).is_none() {
                manager.insert(port, udp::socket::create_socket(port));
                kdebug!("DNS: Socket auto-initialized on port {}", port);
            }
        }

        kdebug!("DNS: resolving {}", domain);

        let dns_addr = NETWORK_INTERFACE.lock().dns.expect("dns not set");
        let id = generate_u16();

        let packet = build_query(domain, id);
        for attempt in 0..3 {
            send_query(dns_addr, &packet);

            if let Some(ip) = wait_for_response(domain, id) {
                kdebug!(
                    "DNS: {} resolved to {}.{}.{}.{} (attempt {})",
                    domain,
                    ip[0],
                    ip[1],
                    ip[2],
                    ip[3],
                    attempt + 1
                );
                return Some(ip);
            }

            kerror!(
                "DNS: timeout resolving {} (attempt {})",
                domain,
                attempt + 1
            );

            sleep(100);
        }

        None
    }
}

fn build_query(domain: &str, id: u16) -> Vec<u8> {
    let label_name = parse_to_label_length(domain);
    let mut packet = alloc::vec![0u8; 512];
    let mut offset = 0;

    let header = DnsHeader::new(id, FLAG_RD);
    let header_bytes = header.as_bytes();
    packet[offset..offset + header_bytes.len()].copy_from_slice(header_bytes);
    offset += header_bytes.len();

    packet[offset..offset + label_name.len()].copy_from_slice(&label_name);
    offset += label_name.len();

    let footer = DnsQuestionFooter {
        qtype: TYPE_A.to_be(),
        qclass: CLASS_IN.to_be(),
    };
    let footer_ptr = &footer as *const _ as *const u8;
    let footer_bytes = unsafe { core::slice::from_raw_parts(footer_ptr, 4) };
    packet[offset..offset + 4].copy_from_slice(footer_bytes);
    offset += 4;

    packet.truncate(offset);
    packet
}

fn send_query(dns_addr: [u8; 4], packet: &[u8]) {
    let socket = get_socket();
    socket.lock().send_to(dns_addr, 53, packet);
}

fn wait_for_response(domain: &str, id: u16) -> Option<[u8; 4]> {
    let socket = get_socket();
    let start_tick = TICKS.load(Ordering::Relaxed);
    let timeout_ticks = 5000;

    loop {
        let mut s = socket.lock();
        while let Some(packet_data) = s.receive() {
            let payload = packet_data.payload;

            let Some(header) = DnsHeader::from_bytes(&payload) else {
                continue;
            };
            if u16::from_be(header.id) != id {
                continue;
            }

            let ancount = u16::from_be(header.ancount);
            if ancount == 0 {
                return None;
            }

            let mut pos = skip_question_section(&payload);

            for _ in 0..ancount {
                let Some((record, next_pos)) = DnsResourceRecord::parse(&payload, pos) else {
                    break;
                };
                let rtype = u16::from_be(record.header.rtype);
                let rdlength = u16::from_be(record.header.rdlength);

                if rtype == TYPE_A && rdlength == 4 {
                    let ip = [
                        record.data[0],
                        record.data[1],
                        record.data[2],
                        record.data[3],
                    ];
                    let ttl = u32::from_be(record.header.ttl);
                    DNS_CACHE.lock().insert(
                        domain.to_string(),
                        DnsCacheEntry {
                            ip,
                            expiration_tick: ttl + TICKS.load(Ordering::Relaxed) as u32,
                        },
                    );
                    return Some(ip);
                }
                pos = next_pos;
            }
        }
        drop(s);

        if TICKS.load(Ordering::Relaxed) - start_tick > timeout_ticks {
            return None;
        }
        sleep(10);
    }
}

fn skip_question_section(payload: &[u8]) -> usize {
    let mut pos = 12;
    while pos < payload.len() && payload[pos] != 0 {
        if (payload[pos] & 0xC0) == 0xC0 {
            pos += 2;
            return pos;
        }
        pos += (payload[pos] as usize) + 1;
    }
    pos += 1; // null terminator
    pos += 4; // qtype + qclass
    pos
}

fn get_socket() -> alloc::sync::Arc<crate::sync::IrqMutex<udp::socket::UdpSocket>> {
    let port = get_dns_port();
    loop {
        let manager = UDP_SOCKET_MANAGER.lock();
        if let Some(socket_arc) = manager.get(&port).cloned() {
            return socket_arc;
        }
        drop(manager);
        sleep(100);
    }
}

fn parse_to_label_length(domain: &str) -> Vec<u8> {
    let mut encoded: Vec<u8> = domain
        .split('.')
        .flat_map(|label| {
            let bytes = label.as_bytes();
            core::iter::once(bytes.len() as u8).chain(bytes.iter().copied())
        })
        .collect();
    encoded.push(0);
    encoded
}
