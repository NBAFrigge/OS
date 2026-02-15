use core::ptr::write_volatile;

use crate::apic::apic::APICPOINTERS;

const SVR_OFFSET: u64 = 0xF0;
const EOI_OFFSET: u64 = 0xB0;

pub unsafe fn init() {
    let svr_addr = APICPOINTERS.lock().get_cpu_apic();

    if svr_addr == 0 {
        return;
    }

    let svr_ptr = (svr_addr + SVR_OFFSET) as *mut u32;
    write_volatile(svr_ptr, 0x1FF);
}

pub unsafe fn send_eoi() {
    let eoi_address = APICPOINTERS.lock().get_cpu_apic();

    if eoi_address == 0 {
        return;
    }

    let eoi_ptr = (eoi_address + EOI_OFFSET) as *mut u32;
    write_volatile(eoi_ptr, 0x0);
}
