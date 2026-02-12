use lazy_static::lazy_static;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

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

catchall_handler!(divide_by_zero_handler);
catchall_handler!(debug_handler);
catchall_handler!(non_maskable_interrupt_handler);
catchall_handler!(breakpoint_handler);
catchall_handler!(overflow_handler);

_double_fault_handler!(double_fault_handler);
_page_fault_handler!(page_fault_handler);

// Test
#[cfg(test)]
use x86_64::instructions::interrupts;

#[test_case]
fn test_breakpoint_exception() {
    interrupts::int3();
}
