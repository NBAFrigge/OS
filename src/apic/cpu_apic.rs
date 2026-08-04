use crate::apic::apic::APICPOINTERS;
use crate::idt::interrupt::TIMER_INTERRUPT_ID;
use core::ptr::{read_volatile, write_volatile};
use x86_64::instructions::port::Port;

const SVR_OFFSET: u64 = 0xF0;
const EOI_OFFSET: u64 = 0xB0;
const LVT_TIME_OFFSET: u64 = 0x320;
const INITIAL_COUNT_OFFSET: u64 = 0x380;
const CURRENT_COUNT_OFFSET: u64 = 0x390;
const DIVIDE_CONF_OFFSET: u64 = 0x3E0;

const DIVIDE_BY_16: u32 = 0x3;
const TIMER_MODE_PERIODIC: u32 = 0x1;
const TIMER_MODE_ONE_SHOT: u32 = 0x0;

pub unsafe fn init() {
    let svr_addr = APICPOINTERS.lock().get_cpu_apic();

    if svr_addr == 0 {
        return;
    }

    // keyboard
    let svr_ptr = (svr_addr + SVR_OFFSET) as *mut u32;
    write_volatile(svr_ptr, 0x1FF);

    // timer
    let lvt_ptr = (svr_addr + LVT_TIME_OFFSET) as *mut u32;
    let initial_count_ptr = (svr_addr + INITIAL_COUNT_OFFSET) as *mut u32;
    let current_count_ptr = (svr_addr + CURRENT_COUNT_OFFSET) as *mut u32;
    let divide_ptr = (svr_addr + DIVIDE_CONF_OFFSET) as *mut u32;

    write_volatile(initial_count_ptr, 0);
    write_volatile(divide_ptr, DIVIDE_BY_16);

    write_volatile(lvt_ptr, get_lvt_value(TIMER_MODE_ONE_SHOT));

    // apic timer calibration
    let mut pit_command = Port::<u8>::new(0x43);
    let mut pit_data = Port::<u8>::new(0x40);
    let latch: u16 = 11932;

    pit_command.write(0x30);
    pit_data.write((latch & 0xFF) as u8);
    pit_data.write(((latch >> 8) & 0xFF) as u8);

    write_volatile(initial_count_ptr, 0xFFFFFFFF);
    let tsc_start = core::arch::x86_64::_rdtsc();

    while (pit_read_status() & 0x80) == 0 {}

    let tsc_end = core::arch::x86_64::_rdtsc();
    let current_apic = read_volatile(current_count_ptr);
    let ticks_in_10ms = 0xFFFFFFFF - current_apic;
    let ticks_per_ms = ticks_in_10ms / 10;

    crate::timer::tsc::set_calibration((tsc_end - tsc_start) / 10_000);

    write_volatile(initial_count_ptr, 0);
    write_volatile(lvt_ptr, get_lvt_value(TIMER_MODE_PERIODIC));
    write_volatile(initial_count_ptr, ticks_per_ms);
}

pub unsafe fn send_eoi() {
    let eoi_address = APICPOINTERS.lock().get_cpu_apic();

    if eoi_address == 0 {
        return;
    }

    let eoi_ptr = (eoi_address + EOI_OFFSET) as *mut u32;
    write_volatile(eoi_ptr, 0x0);
}

unsafe fn pit_read_status() -> u8 {
    let mut cmd = Port::<u8>::new(0x43);
    let mut data = Port::<u8>::new(0x40);
    cmd.write(0xE2);
    data.read()
}

fn get_lvt_value(timer_mode: u32) -> u32 {
    TIMER_INTERRUPT_ID as u32 | (timer_mode << 17)
}
