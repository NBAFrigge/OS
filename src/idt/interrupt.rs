use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt
            .set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
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

catchall_handler!(divide_by_zero_handler);
catchall_handler!(debug_handler);
catchall_handler!(non_maskable_interrupt_handler);
catchall_handler!(breakpoint_handler);
catchall_handler!(overflow_handler);

// TODO: ADD page fault handler

// Test
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
