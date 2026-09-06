use core::ptr::write_volatile;

use crate::{apic::apic::APICPOINTERS, idt::interrupt::KEYBOARD_INTERRUPT_ID};

const IOREGSEL_OFFSET: u64 = 0x00;
const IOWIN_OFFSET: u64 = 0x10;

pub unsafe fn init() {
    let io_ptr = APICPOINTERS.lock().get_io_apic();

    if io_ptr == 0 {
        return;
    }

    let sel_ptr = (io_ptr + IOREGSEL_OFFSET) as *mut u32;
    let win_ptr = (io_ptr + IOWIN_OFFSET) as *mut u32;

    write_volatile(sel_ptr, 0x13);
    write_volatile(win_ptr, 0);
    write_volatile(sel_ptr, 0x12);
    write_volatile(win_ptr, KEYBOARD_INTERRUPT_ID as u32);
}

pub unsafe fn add_redirect(irq: u8, vector: u8) {
    let io_ptr = APICPOINTERS.lock().get_io_apic();
    if io_ptr == 0 {
        return;
    }
    let sel_ptr = (io_ptr + IOREGSEL_OFFSET) as *mut u32;
    let win_ptr = (io_ptr + IOWIN_OFFSET) as *mut u32;

    let low_index = 0x10 + (irq as u32) * 2;
    let high_index = low_index + 1;

    const POLARITY_ACTIVE_LOW: u32 = 1 << 13;
    const TRIGGER_LEVEL: u32 = 1 << 15;
    let low = (vector as u32) | POLARITY_ACTIVE_LOW | TRIGGER_LEVEL;

    write_volatile(sel_ptr, high_index);
    write_volatile(win_ptr, 0);
    write_volatile(sel_ptr, low_index);
    write_volatile(win_ptr, low);
}
