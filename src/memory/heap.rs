use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use crate::memory::frame_allocator::BootInfoFrameAllocator;

const HEAP_START: u64 = 0x_4444_4444_0000;
const SIZE: usize = 1024 * 1024; // 1 MiB
const HEAP_END: u64 = HEAP_START + SIZE as u64 - 1;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init(
    offset_page_table: &mut OffsetPageTable<'_>,
    info_frame_allocator: &mut BootInfoFrameAllocator,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start_page = Page::<Size4KiB>::containing_address(VirtAddr::new(HEAP_START));
        let heap_end_page = Page::<Size4KiB>::containing_address(VirtAddr::new(HEAP_END));
        Page::range_inclusive(heap_start_page, heap_end_page)
    };
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    for page in page_range {
        let frame = info_frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        unsafe {
            offset_page_table
                .map_to(page, frame, flags, info_frame_allocator)?
                .flush();
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, SIZE);
    }

    Ok(())
}
