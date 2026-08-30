use crate::task::task_manager::GLOBAL_TASK_MANAGER;

pub fn cmd_pkill(args: &str) {
    match args.trim().parse::<u8>() {
        Ok(id) => {
            let killed =
                x86_64::instructions::interrupts::without_interrupts(|| {
                    GLOBAL_TASK_MANAGER.lock().kill_task(id)
                });
            if killed {
                println!("task {} terminated", id);
            } else {
                println!("task {} not found", id);
            }
        }
        Err(_) => println!("usage: pkill <id>"),
    }
}
