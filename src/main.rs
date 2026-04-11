#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(naked_functions)]

use core::{ops::Add, panic::PanicInfo};
#[macro_use]
mod vgadriver;
#[macro_use]
mod serial;

use bootloader::{entry_point, BootInfo};
use idt::interrupt;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{shell::shell::shell_task, task::task::idle_task, vgadriver::writer::WRITER};

mod apic;
mod idt;
#[macro_use]
mod timer;
mod command_handler;
mod datetime;
mod drivers;
mod memory;
mod shell;
mod task;

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
    crate::memory::memory::set_physical_memory_offset(boot_info.physical_memory_offset);
    interrupt::init_idt();
    unsafe {
        serial_println!("Loading APIC");
        apic::apic::init(boot_info.physical_memory_offset);
        serial_println!("Loading heap");
        memory::memory::init(&boot_info.memory_map, boot_info.physical_memory_offset);
    }

    command_handler::command_handler::init_commands();

    let idle = crate::task::task::Task::new(0, idle_task as u64);
    let shell = crate::task::task::Task::new(1, shell_task as u64);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = crate::task::task_manager::GLOBAL_TASK_MANAGER.lock();
        manager.task_list.push_back(alloc::boxed::Box::new(idle));
        manager.task_list.push_back(alloc::boxed::Box::new(shell));
        if let Some(first) = manager.task_list.pop_front() {
            manager.current_task = Some(first);
        }
    });

    serial_println!("Setup Finished");
    println!("Kernel Loaded");

    WRITER.lock().redraw_shell_line();

    x86_64::instructions::interrupts::enable();

    #[cfg(test)]
    test_main();

    loop {
        x86_64::instructions::hlt();
    }
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

lazy_static! {
    pub static ref MEMORY_OFFSET: Mutex<u64> = Mutex::new(0);
}
