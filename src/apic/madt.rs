use core::ptr::read_volatile;

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
struct MadtRecordHeader {
    pub entry_type: u8,
    pub record_length: u8,
}

#[repr(C, packed)]
struct MadtLocalApic {
    pub header: MadtRecordHeader, // Type = 0
    pub acpi_processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

#[repr(C, packed)]
struct MadtIoApic {
    pub header: MadtRecordHeader, // Type = 1
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[repr(C, packed)]
struct MadtIso {
    pub header: MadtRecordHeader, // Type = 2
    pub bus_source: u8,
    pub irq_source: u8,
    pub gsi: u32,
    pub flags: u16,
}

impl Madt {
    pub unsafe fn new(virtual_address: u64) -> Madt {
        let madt_talbe: Madt = read_volatile(virtual_address as *const Madt);
        madt_talbe
    }
}
