use crate::task::task_manager::GLOBAL_TASK_MANAGER;

pub fn cmd_ps(_args: &str) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        GLOBAL_TASK_MANAGER.lock().list_tasks();
    });
}
