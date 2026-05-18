use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::mutex::Mutex;

use crate::{
    command_handler::commands::helpers::parse_ip,
    net::ipv4::transport::tcp::{
        self,
        socket::{TcpState, TcpTuple, TCP_SOCKET_MANAGER},
    },
    shell::shell::{HandlerResult, SHELL},
};

struct Context {
    local_port: u16,
    remote_ip: [u8; 4],
    remote_port: u16,
}

pub fn cmd_netcat(args: &str) {
    let mut parts = args.split_whitespace();

    let Some(ip_str) = parts.next() else {
        println!("usage: nc <ip> <port>");
        return;
    };

    let Some(port_str) = parts.next() else {
        println!("usage: nc <ip> <port>");
        return;
    };

    let Some(remote_ip) = parse_ip(ip_str) else {
        println!("invalid ip address");
        return;
    };

    let Ok(remote_port) = port_str.parse::<u16>() else {
        println!("invalid port");
        return;
    };

    let local_port = tcp::socket::connect(&remote_ip, remote_port);
    let mut c = CONTEXT.lock();
    c.local_port = local_port;
    c.remote_ip = remote_ip;
    c.remote_port = remote_port;
    drop(c);

    let mut shell = SHELL.lock();
    shell.on_tick = Some(nc_on_tick);
    shell.on_input = Some(nc_on_input);
    drop(shell);
}

fn nc_on_tick() -> HandlerResult {
    let c = CONTEXT.lock();
    let key = TcpTuple {
        remote_ip: c.remote_ip,
        remote_port: c.remote_port,
        local_port: c.local_port,
    };
    drop(c);

    let manager = TCP_SOCKET_MANAGER.lock();
    if let Some(socket) = manager.get(&key) {
        let mut buf = [0u8; 512];
        let n = socket.lock().read(&mut buf);
        if n > 0 {
            if let Ok(text) = core::str::from_utf8(&buf[..n]) {
                print!("{}", text);
            }
        }

        if socket.lock().state == TcpState::CloseWait {
            let mut n = socket.lock().read(&mut buf);
            while n > 0 {
                if let Ok(text) = core::str::from_utf8(&buf[..n]) {
                    print!("{}", text);
                }
                n = socket.lock().read(&mut buf);
            }
            socket.lock().close();
        }
    }

    HandlerResult::Continue
}

fn nc_on_input(data: &str) -> HandlerResult {
    if data.trim() == "exit" {
        return HandlerResult::Done;
    }
    let c = CONTEXT.lock();
    let key = TcpTuple {
        remote_ip: c.remote_ip,
        remote_port: c.remote_port,
        local_port: c.local_port,
    };
    drop(c);
    let mut payload = Vec::with_capacity(data.len() + 1);
    payload.extend_from_slice(data.as_bytes());
    payload.push(b'\n');

    let manager = TCP_SOCKET_MANAGER.lock();
    if let Some(socket) = manager.get(&key) {
        socket.lock().write(&payload);
    }
    HandlerResult::Continue
}

lazy_static! {
    static ref CONTEXT: Mutex<Context> = Mutex::new(Context {
        local_port: 0,
        remote_ip: [0, 0, 0, 0],
        remote_port: 0,
    });
}
