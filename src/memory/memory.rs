use bootloader::bootinfo;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags},
    VirtAddr,
};

use crate::memory::{
    frame_allocator::{BootInfoFrameAllocator, FRAME_ALLOCATOR},
    heap, paging,
};

pub fn init(memory_map: &'static bootinfo::MemoryMap, offset: u64) {
    let mut offset_page_table = paging::init(offset);

    let info_frame_allocator = BootInfoFrameAllocator::new(memory_map);

    *FRAME_ALLOCATOR.lock() = Some(info_frame_allocator);

    if let Some(ref mut allocator) = *FRAME_ALLOCATOR.lock() {
        heap::init(&mut offset_page_table, allocator).expect("heap initialization failed");
    }

    serial_println!("Memory and Heap initialized successfully");
}
