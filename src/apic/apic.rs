use core::ptr::{self, read_unaligned, read_volatile};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::apic::{apic_pointers, cpu_apic, io_apic, madt, pic};

const START: u64 = 0xE0000;
const END: u64 = 0xFFFFF;
const RSDSING: u64 = 0x2052545020445352; // "RSD PTR " in little-endian
const APICSIGN: &[u8; 4] = b"APIC";

lazy_static! {
    pub static ref APICPOINTERS: Mutex<apic_pointers::ApicPointers> =
        Mutex::new(apic_pointers::ApicPointers::new());
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8, // 0 - 1
    // ACPI 1.0 only
    rsdt_address: u32,

    // ACPI 2.0 only
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
pub struct Rsdt {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,

    pub entries: [u32; 0],
}

impl Rsdt {
    unsafe fn find_madt(rsdt_ptr: *const Rsdt, offset: u64) -> Option<u64> {
        let length = read_unaligned(ptr::addr_of!((*rsdt_ptr).length));
        let num_entries = (length - 36) / 4;

        let entries_ptr = (rsdt_ptr as *const u8).add(36) as *const u32;

        for i in 0..num_entries {
            let entry_phys_addr = read_unaligned(entries_ptr.add(i as usize));
            let table_v_addr = (entry_phys_addr as u64) + offset;

            let signature_ptr = table_v_addr as *const [u8; 4];
            let signature = read_unaligned(signature_ptr);

            if &signature == APICSIGN {
                return Some(table_v_addr);
            }
        }

        None
    }
}

unsafe fn rsdp_scan(offset: u64) -> Option<u64> {
    for pos in (START..END).step_by(16) {
        let v_addr = (pos + offset) as *const u64;

        if read_volatile(v_addr) == RSDSING {
            let rsdp_ptr = v_addr as *const Rsdp;

            if validate_rsdp_checksum(v_addr as *const u8) {
                let rsdp: Rsdp = read_unaligned(rsdp_ptr);
                return Some(rsdp.rsdt_address as u64);
            }
        }
    }
    None
}

unsafe fn validate_rsdp_checksum(ptr: *const u8) -> bool {
    let mut sum: u8 = 0;
    for i in 0..20 {
        sum = sum.wrapping_add(read_volatile(ptr.add(i)));
    }
    sum == 0
}

pub unsafe fn send_eoi() {
    cpu_apic::send_eoi();
}

pub unsafe fn init(offset: u64) {
    pic::disable_8259_pic();

    let rsdt_phys = rsdp_scan(offset).expect("RSDP not found");
    let rsdt_ptr = (rsdt_phys + offset) as *const Rsdt;

    let madt_v_addr = Rsdt::find_madt(rsdt_ptr, offset).expect("APIC table not found");

    let madt_table = madt::Madt::new(madt_v_addr);

    APICPOINTERS
        .lock()
        .set_cpu_apic(madt_table.local_apic_address as u64 + offset);

    for entry in madt_table.entries() {
        match entry.typ {
            0 => unsafe {
                let ptr = entry.base_addr as *const madt::MadtLocalApic;
                let local_apic = read_unaligned(ptr);

                println!(
                    "CPU (Local APIC): ID={}, ProcessorID={}",
                    { local_apic.apic_id },
                    { local_apic.acpi_processor_id }
                );
            },
            1 => unsafe {
                let ptr = entry.base_addr as *const madt::MadtIoApic;
                let io_apic = read_unaligned(ptr);

                APICPOINTERS
                    .lock()
                    .set_io_apic(io_apic.io_apic_address as u64 + offset);

                println!(
                    "I/O APIC: ID={}, Address={:#x}, GSI Base={}",
                    { io_apic.io_apic_id },
                    { io_apic.io_apic_address },
                    { io_apic.global_system_interrupt_base }
                );

                io_apic::init();
            },
            2 => unsafe {
                let ptr = entry.base_addr as *const madt::MadtIso;
                let iso = read_unaligned(ptr);

                println!(
                    "Interrupt Override: Bus={}, IRQ={} -> GSI={}",
                    { iso.bus_source },
                    { iso.irq_source },
                    { iso.gsi }
                );
            },
            _ => {}
        }
        cpu_apic::init();
    }
}
