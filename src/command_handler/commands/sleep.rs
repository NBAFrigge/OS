use crate::task::task::sleep;

pub fn cmd_sleep(args: &str) {
    if let Ok(ms) = args.trim().parse::<u64>() {
        sleep(ms);
    } else {
        println!("command error");
    }
}
