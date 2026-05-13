use crate::idt::interrupt::CTRL_C_PRESSED;
use crate::net::ipv4::transport::udp::socket::{UdpSocket, UDP_SOCKET_MANAGER};
use crate::task::task::sleep;
use core::sync::atomic::Ordering;

pub fn cmd_udplisten(args: &str) {
    let port = match args.trim().parse::<u16>() {
        Ok(p) => p,
        Err(_) => {
            println!("Usage: udplisten <port>");
            return;
        }
    };

    let socket = match UdpSocket::bind(port) {
        Ok(s) => s,
        Err(e) => {
            println!("udplisten: {}", e);
            return;
        }
    };

    CTRL_C_PRESSED.store(false, Ordering::Relaxed);
    println!("Listening on UDP port {} (Ctrl+C to stop)", port);

    loop {
        if CTRL_C_PRESSED.load(Ordering::Relaxed) {
            break;
        }

        if let Some(packet) = socket.lock().receive() {
            let msg = core::str::from_utf8(&packet.payload)
                .unwrap_or("<binary>")
                .trim_end_matches('\n');
            println!(
                "[{}.{}.{}.{}:{}] {}",
                packet.src_ip[0],
                packet.src_ip[1],
                packet.src_ip[2],
                packet.src_ip[3],
                packet.src_port,
                msg
            );
        }

        sleep(1);
    }

    UDP_SOCKET_MANAGER.lock().remove(&port);
    println!("Stopped.");
}
