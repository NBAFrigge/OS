use crate::kinfo;
use bootloader::bootinfo;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags},
    PhysAddr, VirtAddr,
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
        heap::init(&mut offset_page_table, allocator)
            .expect("heap initialization failed");
    }

    kinfo!("Memory and Heap initialized successfully");
}

pub static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);

pub fn set_physical_memory_offset(offset: u64) {
    PHYSICAL_MEMORY_OFFSET.store(offset, Ordering::Relaxed);
}

pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
    let offset = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed);
    PhysAddr::new(virt.as_u64() - offset)
}

pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    let offset = PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed);
    VirtAddr::new(phys.as_u64() + offset)
}
