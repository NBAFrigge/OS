use crate::command_handler::command_handler::COMMAND_REGISTRY;

pub fn cmd_help(_args: &str) {
    for c in COMMAND_REGISTRY.lock().iter() {
        println!("- {}", c.name);
    }
}
