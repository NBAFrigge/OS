use core::ptr;

use crate::apic::madt;

pub struct ApicPointers {
    cpu_apic: *const madt::MadtLocalApic,
    io_apic: *const madt::MadtIoApic,
}

impl ApicPointers {
    pub fn new() -> Self {
        ApicPointers {
            cpu_apic: ptr::null(),
            io_apic: ptr::null(),
        }
    }

    pub fn set_cpu_apic(&mut self, address: u64) {
        self.cpu_apic = address as *const madt::MadtLocalApic;
    }

    pub fn set_io_apic(&mut self, address: u64) {
        self.io_apic = address as *const madt::MadtIoApic;
    }

    pub fn get_cpu_apic(&self) -> u64 {
        self.cpu_apic as u64
    }

    pub fn get_io_apic(&self) -> u64 {
        self.io_apic as u64
    }
}

unsafe impl Send for ApicPointers {}
unsafe impl Sync for ApicPointers {}
