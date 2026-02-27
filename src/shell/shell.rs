use alloc::string::String;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct Shell {
    pub buffer: String,
    pub index: u8,
}

impl Shell {
    fn new() -> Self {
        Shell {
            buffer: String::with_capacity(80),
            index: 0,
        }
    }

    pub fn add_char(&mut self, c: char) {
        if self.index == self.buffer.len() as u8 {
            self.buffer.push(c);
            self.index += 1;
        } else {
            self.buffer.insert(self.index as usize, c);
            self.index += 1;
        }

        serial_println!("current buffer: {}", self.buffer);
    }

    pub fn delete_char(&mut self) {
        if self.index == 0 {
            return;
        }
        self.index -= 1;
        self.buffer.remove(self.index as usize);
        serial_println!("current buffer: {}", self.buffer);
    }

    pub fn move_index_left(&mut self) {
        if self.index == 0 {
            return;
        }
        self.index -= 1;
        serial_println!("current index {}", self.index);
    }

    pub fn move_index_right(&mut self) {
        if self.index >= 80 || self.index + 1 > self.buffer.len() as u8 {
            return;
        }
        self.index += 1;
        serial_println!("current index {}", self.index);
    }
}

lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());
}
