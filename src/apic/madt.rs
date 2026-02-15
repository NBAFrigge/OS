use core::{marker::PhantomData, ptr::read_volatile};

#[repr(C, packed)]
pub struct Madt {
    pub signature: [u8; 4], // "APIC"
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,

    pub local_apic_address: u32,
    pub flags: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct MadtRecordHeader {
    pub entry_type: u8,
    pub record_length: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApic {
    pub header: MadtRecordHeader, // Type = 0
    pub acpi_processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtIoApic {
    pub header: MadtRecordHeader, // Type = 1
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtIso {
    pub header: MadtRecordHeader, // Type = 2
    pub bus_source: u8,
    pub irq_source: u8,
    pub gsi: u32,
    pub flags: u16,
}

pub struct MadtIterator<'a> {
    ptr: *const u8,
    end_ptr: *const u8,
    _phantom: PhantomData<&'a u8>,
}

pub struct MadtEntry {
    pub typ: u8,
    pub len: u8,
    pub base_addr: *const u8,
}

impl<'a> Iterator for MadtIterator<'a> {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr >= self.end_ptr {
            return None;
        }

        unsafe {
            let entry_type = *self.ptr;
            let entry_len = *self.ptr.add(1);

            if entry_len == 0 {
                return None;
            }

            let current_ptr = self.ptr;

            self.ptr = self.ptr.add(entry_len as usize);

            return Some(MadtEntry {
                typ: entry_type,
                len: entry_len,
                base_addr: current_ptr,
            });
        }
    }
}

impl Madt {
    pub unsafe fn new(virtual_address: u64) -> &'static Madt {
        let ptr = virtual_address as *const Madt;
        &*ptr
    }

    pub fn get_local_apic_addr(&self) -> u32 {
        self.local_apic_address
    }

    pub fn entries(&self) -> MadtIterator {
        let start_ptr = unsafe { (self as *const _ as *const u8).add(44) };
        let end_ptr = unsafe { (self as *const _ as *const u8).add(self.length as usize) };

        MadtIterator {
            ptr: start_ptr,
            end_ptr,
            _phantom: PhantomData,
        }
    }
}
