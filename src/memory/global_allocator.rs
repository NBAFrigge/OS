use core::alloc::{GlobalAlloc, Layout};

use crate::memory::{
    buddy_allocator::BUDDYALLOCATOR,
    slab_allocator::{self, SLABALLOCATOR},
};

pub struct GlobalAllocator {}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() <= 1 << (slab_allocator::MAX_ORDERS - 1) {
            return SLABALLOCATOR.lock().alloc(layout.size() as u64);
        } else {
            return BUDDYALLOCATOR.lock().alloc(layout.size() as u64);
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() <= 1 << (slab_allocator::MAX_ORDERS - 1) {
            return SLABALLOCATOR.lock().dealloc(ptr, layout.size() as u64);
        } else {
            return BUDDYALLOCATOR.lock().dealloc(ptr, layout.size() as u64);
        }
    }
}

#[global_allocator]
pub static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator {};
