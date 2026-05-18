#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(naked_functions)]

use core::panic::PanicInfo;
#[macro_use]
mod vgadriver;
#[macro_use]
mod serial;
#[macro_use]
mod logger;
mod crypto;
mod net;

use bootloader::{entry_point, BootInfo};
use idt::interrupt;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    crypto::random::GLOBAL_ENTROPY,
    net::{
        dispatcher::network_task_entry,
        e1000::{E1000, E1000_DRIVER},
        interface::NETWORK_INTERFACE,
        ipv4::transport::udp::dhcp::protocol::dhcp_task,
    },
    shell::shell::shell_task,
    task::task::{idle_task, Task},
    vgadriver::writer::WRITER,
};

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
    kinfo!("Kernel started");
    crate::memory::memory::set_physical_memory_offset(boot_info.physical_memory_offset);
    interrupt::init_idt();

    unsafe {
        kinfo!("Loading APIC");
        apic::apic::init(boot_info.physical_memory_offset);
        kinfo!("Loading heap");
        memory::memory::init(&boot_info.memory_map, boot_info.physical_memory_offset);
    }

    command_handler::command_handler::init_commands();

    let e1000 = E1000::init().unwrap_or_else(|err| {
        kerror!("E1000 init failed: {}", err);
        panic!("E1000 init failed");
    });
    *E1000_DRIVER.lock() = Some(e1000);

    if let Some(ref nic) = *E1000_DRIVER.lock() {
        NETWORK_INTERFACE.lock().hw_addr = Some(nic.mac)
    } else {
        println!("Error: NIC not initialized");
    };
    NETWORK_INTERFACE.lock().ip_addr = Some([0, 0, 0, 0]);
    NETWORK_INTERFACE.lock().subnet_mask = Some([0, 0, 0, 0]);
    NETWORK_INTERFACE.lock().gateway_ip = Some([0, 0, 0, 0]);
    NETWORK_INTERFACE.lock().dns = Some([0, 0, 0, 0]);

    let idle = Task::new(0, idle_task as u64);
    let shell = Task::new(1, shell_task as u64);
    let network_poll = Task::new(2, network_task_entry as u64);
    let dhcp = Task::new(3, dhcp_task as u64);

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = crate::task::task_manager::GLOBAL_TASK_MANAGER.lock();
        manager.task_list.push_back(alloc::boxed::Box::new(idle));
        manager.task_list.push_back(alloc::boxed::Box::new(shell));
        manager
            .task_list
            .push_back(alloc::boxed::Box::new(network_poll));
        manager.task_list.push_back(alloc::boxed::Box::new(dhcp));

        if let Some(first) = manager.task_list.pop_front() {
            manager.current_task = Some(first);
        }
    });

    {
        let mut pool = GLOBAL_ENTROPY.lock();
        pool.init();
    }

    kinfo!("Entropy pool initialized.");

    kinfo!("Setup Finished");
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
