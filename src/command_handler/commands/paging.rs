use crate::memory::{buddy_allocator::ALLOCATOR, frame_allocator::FRAME_ALLOCATOR};

pub fn cmd_paging(_args: &str) {
    if let Some(allocator) = &*FRAME_ALLOCATOR.lock() {
        let used_frames = allocator.get_used_frames();
        let used_bytes = used_frames * 4096;
        let free_bytes = ALLOCATOR.lock().total_free_bytes;
        println!("Paging Statistics:");
        println!("Used Frames: {}", used_frames);
        println!("Used Memory: {} KB", used_bytes / 1024);
        println!("Free memory: {} KB", free_bytes / 1024);
        println!(
            "Heap size: {} KB",
            (used_bytes + free_bytes as usize) / 1024
        );
    } else {
        println!("Error: Frame Allocator not initialized.");
    }
}
