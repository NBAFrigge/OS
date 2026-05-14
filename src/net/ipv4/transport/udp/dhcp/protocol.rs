use core::sync::atomic::Ordering;

use crate::{
    crypto::random::generate_u32,
    idt::interrupt::TICKS,
    kdebug, kerror,
    net::{
        interface::NETWORK_INTERFACE,
        ipv4::transport::udp::{
            self,
            dhcp::{
                self,
                packet::{
                    constants::{DHCPACK, DHCPOFFER, OPT_MESSAGE_TYPE, OPT_SERVER_IDENTIFIER},
                    DhcpPacket,
                },
            },
            socket::{self, UDP_SOCKET_MANAGER},
        },
    },
    task::task::sleep,
};

fn get_socket() -> alloc::sync::Arc<spin::Mutex<udp::socket::UdpSocket>> {
    loop {
        let mut manager = UDP_SOCKET_MANAGER.lock();
        if let Some(socket_arc) = manager.get(&68).cloned() {
            return socket_arc;
        }
        drop(manager);
        sleep(100);
    }
}

fn create() {
    let mut manager = UDP_SOCKET_MANAGER.lock();
    if manager.get(&68).is_none() {
        manager.insert(68, udp::socket::create_socket(68));
    }
}

fn discover(xid: u32) {
    let socket = get_socket();
    let discover = dhcp::packet::DhcpPacket::new_discover(xid);
    let data = discover.as_bytes();
    socket.lock().send_to([255, 255, 255, 255], 67, data);
}

fn handle_offer(expected_xid: u32) -> Option<([u8; 4], [u8; 4])> {
    let socket = get_socket();
    let start_tick = TICKS.load(Ordering::Relaxed);
    let timeout_ticks = 5000;

    loop {
        let mut s = socket.lock();
        while let Some(packet_data) = s.receive() {
            if let Some(dhcp) = DhcpPacket::from_bytes(&packet_data.payload) {
                if dhcp.header.xid == expected_xid.to_be() {
                    if let Some(msg_type) = dhcp.get_option(OPT_MESSAGE_TYPE) {
                        if msg_type[0] == DHCPOFFER {
                            let offered_ip = dhcp.header.yiaddr;
                            if let Some(server_id_bytes) = dhcp.get_option(OPT_SERVER_IDENTIFIER) {
                                let server_ip = [
                                    server_id_bytes[0],
                                    server_id_bytes[1],
                                    server_id_bytes[2],
                                    server_id_bytes[3],
                                ];
                                return Some((offered_ip, server_ip));
                            }
                        }
                    }
                }
            }
        }
        drop(s);

        if TICKS.load(Ordering::Relaxed) - start_tick > timeout_ticks {
            return None;
        }
        sleep(10);
    }
}

fn request(xid: u32, requested_ip: &[u8; 4], server_ip: &[u8; 4]) {
    let socket = get_socket();
    let request = DhcpPacket::new_request(xid, requested_ip, server_ip);
    let data = request.as_bytes();
    socket.lock().send_to([255, 255, 255, 255], 67, data);
}

fn handle_ack(expected_xid: u32) -> Option<([u8; 4], [u8; 4], [u8; 4], [u8; 4])> {
    let socket = get_socket();
    let start_tick = TICKS.load(Ordering::Relaxed);
    let timeout_ticks = 5000;

    loop {
        let mut s = socket.lock();
        while let Some(packet_data) = s.receive() {
            if let Some(dhcp) = DhcpPacket::from_bytes(&packet_data.payload) {
                if dhcp.header.xid == expected_xid.to_be() {
                    if let Some(msg_type) = dhcp.get_option(OPT_MESSAGE_TYPE) {
                        if msg_type[0] == DHCPACK {
                            let assigned_ip = dhcp.header.yiaddr;
                            let subnet_mask = dhcp
                                .get_option(1)
                                .map(|b| [b[0], b[1], b[2], b[3]])
                                .unwrap_or([255, 255, 255, 0]);
                            let gateway = dhcp
                                .get_option(3)
                                .map(|b| [b[0], b[1], b[2], b[3]])
                                .unwrap_or([0, 0, 0, 0]);
                            let dns = dhcp
                                .get_option(6)
                                .map(|b| [b[0], b[1], b[2], b[3]])
                                .unwrap_or([8, 8, 8, 8]);

                            return Some((assigned_ip, subnet_mask, gateway, dns));
                        }
                    }
                }
            }
        }
        drop(s);

        if TICKS.load(Ordering::Relaxed) - start_tick > timeout_ticks {
            return None;
        }
        sleep(10);
    }
}

pub fn dhcp_task() {
    create();

    loop {
        kdebug!("DHCP: starting DORA");
        let xid = generate_u32();
        discover(xid);

        if let Some((requested_ip, server_ip)) = handle_offer(xid) {
            kdebug!(
                "DHCP: offer received {}.{}.{}.{}",
                requested_ip[0],
                requested_ip[1],
                requested_ip[2],
                requested_ip[3]
            );

            request(xid, &requested_ip, &server_ip);

            if let Some((assigned_ip, subnet_mask, gateway, dns)) = handle_ack(xid) {
                let mut interface = NETWORK_INTERFACE.lock();
                interface.ip_addr = Some(assigned_ip);
                interface.subnet_mask = Some(subnet_mask);
                interface.gateway_ip = Some(gateway);
                interface.dns = Some(dns);
                drop(interface);

                kdebug!("DHCP: Network fully configured!");
                kdebug!(
                    "IP:      {}.{}.{}.{}",
                    assigned_ip[0],
                    assigned_ip[1],
                    assigned_ip[2],
                    assigned_ip[3]
                );
                kdebug!(
                    "Mask:    {}.{}.{}.{}",
                    subnet_mask[0],
                    subnet_mask[1],
                    subnet_mask[2],
                    subnet_mask[3]
                );
                kdebug!(
                    "Gateway: {}.{}.{}.{}",
                    gateway[0],
                    gateway[1],
                    gateway[2],
                    gateway[3]
                );
                kdebug!("DNS:     {}.{}.{}.{}", dns[0], dns[1], dns[2], dns[3]);
                break;
            } else {
                kerror!("DHCP: ACK timeout, retrying...");
            }
        } else {
            kerror!("DHCP: offer timeout, retrying...");
        }
        sleep(2000);
    }

    loop {
        sleep(10000);
    }
}
