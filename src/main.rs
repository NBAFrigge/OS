#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
#[macro_use]
mod vgadriver;
#[macro_use]
mod serial;

use bootloader::{entry_point, BootInfo};
use idt::interrupt;

mod apic;
mod idt;
#[macro_use]
mod timer;
mod command_handler;
mod datetime;
mod memory;
mod shell;

extern crate alloc;

pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

entry_point!(kernel_main);

#[no_mangle]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    serial_println!("Kernel started");

    interrupt::init_idt();
    unsafe {
        serial_println!("Loading APIC");
        apic::apic::init(boot_info.physical_memory_offset);
        serial_println!("Loading heap");
        memory::memory::init(&boot_info.memory_map, boot_info.physical_memory_offset);
    }

    serial_println!("Setup Finished");
    println!("Kernel Loaded");
    x86_64::instructions::interrupts::enable();

    command_handler::command_handler::init_commands();

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[failed]\n");
    println!("Error: {}\n", info);
    loop {}
}
