use core::ptr::{self, read_unaligned, read_volatile};

use crate::apic::madt;

const START: u64 = 0xE0000;
const END: u64 = 0xFFFFF;
const RSDSING: u64 = 0x2052545020445352; // "RSD PTR " in little-endian
const APICSIGN: &[u8; 4] = b"APIC";

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
        // Usa read_unaligned perché rsdt_ptr è packed
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

pub unsafe fn init(offset: u64) {
    let rsdt_phys = rsdp_scan(offset).expect("RSDP not found");
    let rsdt_ptr = (rsdt_phys + offset) as *const Rsdt;

    let madt_v_addr = Rsdt::find_madt(rsdt_ptr, offset).expect("APIC table not found");

    let madt_table = madt::Madt::new(madt_v_addr);
    println!("MADT Revision: {}", madt_table.revision);
}
