use core::arch::naked_asm;
use core::sync::atomic::{AtomicBool, AtomicU64};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::apic::apic::send_eoi;
use crate::crypto::random::{EntropyPool, GLOBAL_ENTROPY};
use crate::shell::shell::{PENDING_COMMAND, SHELL};
use crate::task::task_manager::GLOBAL_TASK_MANAGER;
use crate::vgadriver::keymap::KEYMAPDRIVER;
use crate::vgadriver::writer::WRITER;

pub const KEYBOARD_INTERRUPT_ID: u8 = 33;
pub const TIMER_INTERRUPT_ID: u8 = 34;
pub static TICKS: AtomicU64 = AtomicU64::new(0);
pub static CTRL_C_PRESSED: AtomicBool = AtomicBool::new(false);
static CTRL_HELD: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref IDT: Mutex<InterruptDescriptorTable> = {
        let idt = Mutex::new(InterruptDescriptorTable::new());
        let mut locked_idt = idt.lock();
        locked_idt
            .divide_error
            .set_handler_fn(divide_by_zero_handler);
        locked_idt.debug.set_handler_fn(debug_handler);
        locked_idt
            .non_maskable_interrupt
            .set_handler_fn(non_maskable_interrupt_handler);
        locked_idt.breakpoint.set_handler_fn(breakpoint_handler);
        locked_idt.overflow.set_handler_fn(overflow_handler);
        locked_idt.double_fault.set_handler_fn(double_fault_handler);
        locked_idt.page_fault.set_handler_fn(page_fault_handler);
        locked_idt[KEYBOARD_INTERRUPT_ID as usize].set_handler_fn(keyboard_handler);
        locked_idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_handler);
        drop(locked_idt);
        idt
    };
}

pub fn init_idt() {
    let idt = IDT.lock();
    unsafe {
        idt.load_unsafe();
    };
    drop(idt)
}

pub fn add_handler(index_id: usize, f: extern "x86-interrupt" fn(InterruptStackFrame)) {
    let mut idt = IDT.lock();
    idt[index_id].set_handler_fn(f);
    unsafe { idt.load_unsafe() }
    drop(idt);
}

macro_rules! catchall_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame) {
            println!("EXCEPTION: {}\n{:#?}", stringify!($name), stack_frame);
        }
    };
}

macro_rules! _double_fault_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
            println!(
                "EXCEPTION DOUBLE FAULT [{}]: {}\n{:#?}",
                error_code,
                stringify!($name),
                stack_frame
            );
            loop {}
        }
    };
}

macro_rules! _page_fault_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(
            stack_frame: InterruptStackFrame,
            error_code: PageFaultErrorCode,
        ) {
            let failed_reg = Cr2::read().as_u64();

            println!(
                "EXCEPTION PAGE FAULT:\nError Code: {:?}\nAccessed Address: {:#x}\n{:#?}",
                error_code, failed_reg, stack_frame
            );
            loop {}
        }
    };
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    match scancode {
        0x1D => CTRL_HELD.store(true, core::sync::atomic::Ordering::Relaxed),
        0x9D => CTRL_HELD.store(false, core::sync::atomic::Ordering::Relaxed),
        0x2E if CTRL_HELD.load(core::sync::atomic::Ordering::Relaxed) => {
            CTRL_C_PRESSED.store(true, core::sync::atomic::Ordering::Relaxed);
        }
        0x4B => {
            // right arrow
            SHELL.lock().move_index_left();
            WRITER.lock().redraw_shell_line();
        }
        0x4D => {
            // left arrow
            SHELL.lock().move_index_right();
            WRITER.lock().redraw_shell_line();
        }
        _ => {
            let c = KEYMAPDRIVER.lock().convert(scancode);
            if c != '\0' {
                if c == '\x08' {
                    // backspace
                    SHELL.lock().delete_char();
                    WRITER.lock().redraw_shell_line();
                } else if c == '\n' {
                    println!();

                    let command_input = {
                        let mut shell = SHELL.lock();
                        shell.send_buffer()
                    };
                    *PENDING_COMMAND.lock() = Some(command_input);
                } else {
                    SHELL.lock().add_char(c);
                    WRITER.lock().redraw_shell_line();
                }
            }
        }
    }

    let tsc = unsafe { core::arch::x86_64::_rdtsc() };

    if let Some(mut pool) = GLOBAL_ENTROPY.try_lock() {
        let noise = tsc ^ (scancode as u64);
        pool.add_noise(noise);
    }

    unsafe {
        send_eoi();
    };
}

#[unsafe(naked)]
extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        naked_asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rbx",
            "push rbp",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "mov rdi, rsp",
            "call timer_tick_handler",
            "mov rsp, rax",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rbp",
            "pop rbx",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq",
        );
    }
}

#[no_mangle]
extern "C" fn timer_tick_handler(stack_pointer: usize) -> usize {
    TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    unsafe {
        crate::apic::apic::send_eoi();
    }

    crate::net::dispatcher::poll_network();
    crate::net::ipv4::transport::tcp::socket::tcp_tick();
    GLOBAL_TASK_MANAGER.lock().rotate(stack_pointer)
}

catchall_handler!(divide_by_zero_handler);
catchall_handler!(debug_handler);
catchall_handler!(non_maskable_interrupt_handler);
catchall_handler!(breakpoint_handler);
catchall_handler!(overflow_handler);

_double_fault_handler!(double_fault_handler);
_page_fault_handler!(page_fault_handler);

// test
#[cfg(test)]
use x86_64::instructions::interrupts;

#[test_case]
fn test_breakpoint_exception() {
    interrupts::int3();
}
