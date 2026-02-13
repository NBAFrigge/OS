use core::ptr::{self, read_volatile};

const START: u64 = 0xE0000;
const END: u64 = 0xFFFFF;
const RSDSING: u64 = 0x2052545020445352; // "RSD PTR " in little-endian

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

pub unsafe fn rsdp_scan(offset: u64) -> Option<u64> {
    for pos in (START..END).step_by(16) {
        let v_addr = (pos + offset) as *const u64;
        if read_volatile(v_addr) == RSDSING {
            let rsdp_ptr = v_addr as *const Rsdp;

            if validate_rsdp_checksum(v_addr as *const u8) {
                let rsdp: Rsdp = read_volatile(rsdp_ptr);
                if rsdp.revision >= 2 {
                    // TODO: xsdt handling
                    return Some(rsdp.xsdt_address);
                } else {
                    return Some(rsdp.rsdt_address as u64);
                }
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

