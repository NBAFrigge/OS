use core::{ptr::null, sync::atomic::Ordering};

use alloc::string::String;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{
    command_handler::command_handler::run_command,
    idt::interrupt::CTRL_C_PRESSED,
    kdebug,
    shell::{self, parser::parser},
    vgadriver::writer::WRITER,
};

#[derive(PartialEq, Eq)]
pub enum HandlerResult {
    Continue,
    Done,
}

pub struct Shell {
    pub buffer: String,
    pub index: u8,
    pub on_tick: Option<fn() -> HandlerResult>,
    pub on_input: Option<fn(&str) -> HandlerResult>,
    pub on_close: Option<fn() -> HandlerResult>,
}

impl Shell {
    fn new() -> Self {
        Shell {
            buffer: String::with_capacity(80),
            index: 0,
            on_tick: None,
            on_input: None,
            on_close: None,
        }
    }

    pub fn add_char(&mut self, c: char) {
        if self.buffer.len() < 79 {
            if self.index == self.buffer.len() as u8 {
                self.buffer.push(c);
                self.index += 1;
            } else {
                self.buffer.insert(self.index as usize, c);
                self.index += 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        if self.index == 0 {
            return;
        }
        self.index -= 1;
        self.buffer.remove(self.index as usize);
    }

    pub fn move_index_left(&mut self) {
        if self.index == 0 {
            return;
        }
        self.index -= 1;
    }

    pub fn move_index_right(&mut self) {
        if self.index >= 79 || self.index + 1 > self.buffer.len() as u8 {
            return;
        }
        self.index += 1;
    }

    pub fn send_buffer(&mut self) -> String {
        let input = core::mem::take(&mut self.buffer);
        self.index = 0;
        input
    }
}

pub extern "C" fn shell_task() -> ! {
    loop {
        let tick = without_interrupts(|| SHELL.lock().on_tick);
        let on_input = without_interrupts(|| SHELL.lock().on_input);
        let on_close = without_interrupts(|| SHELL.lock().on_close);

        if let Some(tick_fn) = tick {
            if tick_fn() == HandlerResult::Done || CTRL_C_PRESSED.load(Ordering::Relaxed) {
                kdebug!("nc: exit signal recieved");
                CTRL_C_PRESSED.store(false, Ordering::Relaxed);
                without_interrupts(|| {
                    let mut shell = SHELL.lock();
                    if let Some(on_close_fn) = on_close {
                        on_close_fn();
                    }
                    shell.on_tick = None;
                    shell.on_input = None;
                    shell.on_close = None;
                });
                without_interrupts(|| {
                    WRITER.lock().redraw_shell_line();
                });
            }
        }

        let cmd =
            x86_64::instructions::interrupts::without_interrupts(|| PENDING_COMMAND.lock().take());

        if let Some(cmd) = cmd {
            if !cmd.trim().is_empty() {
                if let Some(on_input_fn) = on_input {
                    if on_input_fn(&cmd) == HandlerResult::Done {
                        let mut shell = SHELL.lock();
                        if let Some(on_close_fn) = on_close {
                            on_close_fn();
                        }
                        shell.on_tick = None;
                        shell.on_input = None;
                        shell.on_close = None;
                    }
                } else if let Some((c, args)) = parser(&cmd) {
                    run_command(c, args);
                } else {
                    run_command(cmd.trim(), "");
                }
            }
            x86_64::instructions::interrupts::without_interrupts(|| {
                WRITER.lock().redraw_shell_line();
            });
        }

        x86_64::instructions::hlt();
    }
}

lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());
    pub static ref PENDING_COMMAND: Mutex<Option<String>> = Mutex::new(None);
}
