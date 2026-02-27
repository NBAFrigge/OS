use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use lazy_static::lazy_static;
use spin::mutex::Mutex;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

#[derive(Debug, Copy, Clone)]
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub fn new(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    pub fn get_used_frames(self) -> usize {
        self.next
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let address = self
            .memory_map
            .iter()
            .filter(|p| p.region_type == MemoryRegionType::Usable)
            .flat_map(|p| (p.range.start_addr()..p.range.end_addr()).step_by(4096))
            .nth(self.next);

        if address == None {
            return None;
        }
        let phys_addr = PhysAddr::new(address.unwrap());
        self.next += 1;
        Some(PhysFrame::containing_address(phys_addr))
    }
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);
}
