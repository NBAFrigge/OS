use core::fmt;
use lazy_static::lazy_static;
use volatile::Volatile;

use crate::sync::IrqMutex;

use crate::shell::shell::SHELL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: u8,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const NEW_LINE: u8 = 0x0A;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: u8,
    buffer: &'static mut Buffer,
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Writer {
    pub fn new() -> Self {
        let mut w = Writer {
            column_position: 0,
            row_position: 0,
            color_code: 0x07,
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        };
        w.clear_screen();
        w.update_cursor();
        w
    }

    fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        if byte == NEW_LINE {
            self.new_line();
            return;
        }

        if self.column_position >= BUFFER_WIDTH {
            self.new_line();
        }

        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character: byte,
            color_code: self.color_code,
        });

        self.column_position += 1;
        self.update_cursor();
    }

    fn new_line(&mut self) {
        self.column_position = 0;

        if self.row_position >= BUFFER_HEIGHT - 1 {
            self.scroll();
        } else {
            self.row_position += 1;
        }
        self.update_cursor();
    }

    fn scroll(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
        self.update_cursor();
    }

    pub fn backspace(&mut self) {
        if self.column_position > 0 {
            self.column_position -= 1;

            let blank = ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            };
            self.buffer.chars[self.row_position][self.column_position].write(blank);

            self.update_cursor();
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn clear(&mut self) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row][col].write(blank);
            }
        }
        self.column_position = 0;
        self.row_position = 0;
        self.update_cursor();
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    pub fn redraw_shell_line(&mut self) {
        let mut print_bash = true;
        let (content, shell_index) = {
            let shell = SHELL.lock();
            if shell.on_tick != None {
                print_bash = false;
            }
            (shell.buffer.clone(), shell.index as usize)
        };

        self.clear_row(self.row_position);

        self.column_position = 0;
        if print_bash {
            self.write_byte_raw(b'>');
        }

        for byte in content.bytes() {
            self.write_byte_raw(byte);
        }

        self.column_position = shell_index + if print_bash { 1 } else { 0 };
        self.update_cursor();
    }

    fn write_byte_raw(&mut self, byte: u8) {
        if self.column_position >= BUFFER_WIDTH {
            return;
        }

        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character: byte,
            color_code: self.color_code,
        });
        self.column_position += 1;
    }
    fn update_cursor(&mut self) {
        let mut pos = (self.row_position * BUFFER_WIDTH) + self.column_position;

        if pos >= BUFFER_WIDTH * BUFFER_HEIGHT {
            pos = (BUFFER_WIDTH * BUFFER_HEIGHT) - 1;
        }

        unsafe {
            let mut addr_port = x86_64::instructions::port::Port::<u8>::new(0x3D4);
            let mut data_port = x86_64::instructions::port::Port::<u8>::new(0x3D5);

            addr_port.write(0x0E);
            data_port.write((pos >> 8) as u8);
            addr_port.write(0x0F);
            data_port.write((pos & 0xFF) as u8);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.column_position == 0 {
            return;
        }
        self.column_position -= 1;
        self.update_cursor();
    }

    pub fn move_cursor_right(&mut self) {
        if self.column_position >= 80 {
            return;
        }
        self.column_position += 1;
        self.update_cursor();
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vgadriver::writer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

lazy_static! {
    pub static ref WRITER: IrqMutex<Writer> = IrqMutex::new(Writer::new());
}
