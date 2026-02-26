use bootloader::bootinfo;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags},
    VirtAddr,
};

use crate::memory::{frame_allocator::BootInfoFrameAllocator, heap, paging};

pub fn init(memory_map: &'static bootinfo::MemoryMap, offset: u64) {
    let mut offset_page_table = paging::init(offset);
    let mut info_frame_allocator = BootInfoFrameAllocator::new(memory_map);

    heap::init(&mut offset_page_table, &mut info_frame_allocator)
        .expect("heap initialization failed");

    serial_println!("Memory and Heap initialized successfully");
}
