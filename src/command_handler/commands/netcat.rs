use alloc::vec::Vec;
use core::net::IpAddr;

use crate::shell::shell::{HandlerResult, SHELL};

pub fn cmd_netcat(args: &str) {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    let mut parts = args.split_whitespace();

    let Some(ip_str) = parts.next() else {
        println!("usage: nc <ip> <port>");
        return;
    };

    let Some(port_str) = parts.next() else {
        return;
    };

    let Ok(ip) = ip_str.parse::<IpAddr>() else {
        return;
    };

    let Ok(port) = port_str.parse::<u16>() else {
        return;
    };

    let mut shell = SHELL.lock();
    shell.on_tick = Some(nc_on_tick);
    shell.on_input = Some(nc_on_input);
}

fn nc_on_tick() -> HandlerResult {
    return HandlerResult::Continue;
}

fn nc_on_input(data: &str) -> HandlerResult {
    println!("received {}", data);
    return HandlerResult::Continue;
}
