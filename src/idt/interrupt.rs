use core::sync::atomic::AtomicU64;

use lazy_static::lazy_static;
use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::apic::apic::send_eoi;
use crate::shell::shell::SHELL;
use crate::vgadriver::keymap::KEYMAPDRIVER;
use crate::vgadriver::writer::WRITER;

pub const KEYBOARD_INTERRUPT_ID: u8 = 33;
pub const TIMER_INTERRUPT_ID: u8 = 34;
pub static TICKS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt
            .set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[KEYBOARD_INTERRUPT_ID as usize].set_handler_fn(keyboard_handler);
        idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
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
                    SHELL.lock().buffer.clear();
                    SHELL.lock().index = 0;
                } else {
                    SHELL.lock().add_char(c);
                    WRITER.lock().redraw_shell_line();
                }
            }
        }
    }

    unsafe {
        send_eoi();
    };
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    unsafe {
        send_eoi();
    }
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
