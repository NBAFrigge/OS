use alloc::vec::Vec;
use core::arch::asm;
use x86_64::structures::idt::InterruptStackFrame;

use crate::idt::interrupt::TICKS;

const STACK_CAPACITY: usize = 1024 * 16; // 16 kib

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Ready,
    Running,
    Waiting,
}

pub struct Task {
    pub id: u8,
    pub state: State,
    pub saved_stack_pointer: usize,
    pub stack: Vec<u8>,
    pub wake_on_tick: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Registers {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,

    // CPU handled
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl Task {
    pub fn new(id: u8, entry_point: u64) -> Self {
        let mut stack = Vec::with_capacity(STACK_CAPACITY);

        let stack_start = stack.as_ptr() as usize;
        let stack_top = stack_start + STACK_CAPACITY;

        let registers_size = core::mem::size_of::<Registers>();
        let saved_sp = (stack_top - registers_size) as *mut Registers;

        unsafe {
            let regs = &mut *saved_sp;

            core::ptr::write_bytes(saved_sp, 0, 1);

            regs.rip = entry_point;
            regs.cs = 8;
            regs.rflags = 0x202; // interrupt enabled
            regs.rsp = stack_top as u64;
            regs.ss = 0;

            Task {
                id,
                state: State::Ready,
                saved_stack_pointer: saved_sp as usize,
                stack,
                wake_on_tick: 0,
            }
        }
    }
}

pub fn yield_now() {
    unsafe {
        asm!("int 34");
    }
}

pub fn sleep(ms: u64) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut manager = crate::task::task_manager::GLOBAL_TASK_MANAGER.lock();

        if let Some(task) = manager.current_task.as_mut() {
            task.state = State::Waiting;
            task.wake_on_tick =
                crate::idt::interrupt::TICKS.load(core::sync::atomic::Ordering::Relaxed) + ms;
        }
    });

    yield_now();
}

pub extern "C" fn idle_task() -> ! {
    loop {
        unsafe {
            x86_64::instructions::hlt();
        }
    }
}
