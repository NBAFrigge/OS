use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

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
