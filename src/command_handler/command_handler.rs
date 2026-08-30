use alloc::string::String;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::command_handler::commands::arp_test::cmd_arp_test;
use crate::command_handler::commands::bench::cmd_bench;
use crate::command_handler::commands::clear::cmd_clear;
use crate::command_handler::commands::curl::cmd_curl;
use crate::command_handler::commands::echo::cmd_echo;
use crate::command_handler::commands::help::cmd_help;
use crate::command_handler::commands::loglevel::{
    cmd_get_loglevel, cmd_set_loglevel,
};
use crate::command_handler::commands::lspci::cmd_lspci;
use crate::command_handler::commands::netcat::cmd_netcat;
use crate::command_handler::commands::paging::cmd_paging;
use crate::command_handler::commands::ping::cmd_ping;
use crate::command_handler::commands::pkill::cmd_pkill;
use crate::command_handler::commands::ps::cmd_ps;
use crate::command_handler::commands::sleep::cmd_sleep;
use crate::command_handler::commands::udplisten::cmd_udplisten;

type CommandFn = fn(&str);

pub struct Command {
    pub name: &'static str,
    pub func: CommandFn,
}

pub fn init_commands() {
    let mut registry = COMMAND_REGISTRY.lock();
    registry.push(Command {
        name: "help",
        func: cmd_help,
    });
    registry.push(Command {
        name: "echo",
        func: cmd_echo,
    });
    registry.push(Command {
        name: "paging",
        func: cmd_paging,
    });
    registry.push(Command {
        name: "clear",
        func: cmd_clear,
    });
    registry.push(Command {
        name: "sleep",
        func: cmd_sleep,
    });
    registry.push(Command {
        name: "lspci",
        func: cmd_lspci,
    });
    registry.push(Command {
        name: "arptest",
        func: cmd_arp_test,
    });
    registry.push(Command {
        name: "ping",
        func: cmd_ping,
    });
    registry.push(Command {
        name: "udplisten",
        func: cmd_udplisten,
    });
    registry.push(Command {
        name: "setloglevel",
        func: cmd_set_loglevel,
    });
    registry.push(Command {
        name: "getloglevel",
        func: cmd_get_loglevel,
    });
    registry.push(Command {
        name: "ps",
        func: cmd_ps,
    });
    registry.push(Command {
        name: "pkill",
        func: cmd_pkill,
    });
    registry.push(Command {
        name: "nc",
        func: cmd_netcat,
    });
    registry.push(Command {
        name: "curl",
        func: cmd_curl,
    });
    registry.push(Command {
        name: "bench",
        func: cmd_bench,
    });
}

pub fn run_command(name: &str, args: &str) {
    let cmd_func = {
        let registry = COMMAND_REGISTRY.lock();
        registry.iter().find(|c| c.name == name).map(|c| c.func)
    };

    match cmd_func {
        Some(func) => (func)(args),
        None => {
            if !name.is_empty() {
                println!("command not found {}", name)
            }
        }
    }
}
lazy_static! {
    pub static ref COMMAND_REGISTRY: Mutex<Vec<Command>> =
        Mutex::new(Vec::new());
}
