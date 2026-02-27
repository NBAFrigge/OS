use crate::memory::frame_allocator::FRAME_ALLOCATOR;

pub fn cmd_paging(_args: &str) {
    if let Some(allocator) = &*FRAME_ALLOCATOR.lock() {
        let used_frames = allocator.get_used_frames();
        let used_bytes = used_frames * 4096;

        println!("Paging Statistics:");
        println!("Used Frames: {}", used_frames);
        println!("Used Memory: {} KB", used_bytes / 1024);
    } else {
        println!("Error: Frame Allocator not initialized.");
    }
}
