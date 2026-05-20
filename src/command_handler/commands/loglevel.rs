use alloc::vec::Vec;

use crate::logger::logger::{LogLevel, LOGGER};

pub fn cmd_get_loglevel(_args: &str) {
    let current = LOGGER.get_log_level();
    println!("log level: {}", current.as_str());
}

pub fn cmd_set_loglevel(args: &str) {
    let args_list: Vec<&str> = args.split_whitespace().collect();
    match args_list.len() {
        0 => {
            let current = LOGGER.get_log_level();
            println!("Current log level: {}", current.as_str());
        }
        1 => {
            if let Some(level) = LogLevel::from_str(args_list[0]) {
                LOGGER.set_log_level(level);
                println!("Log level set to: {}", level.as_str());
            } else {
                println!("Error: Invalid log level '{}'", args_list[0]);
                println!("Available levels: error, warn, info, debug, trace");
            }
        }
        _ => {
            println!("Usage: loglevel [level]");
        }
    }
}
