use lazy_static::lazy_static;
use spin::Mutex;

const LEFT_SHIFT_PRESS: u8 = 0x2A;
const LEFT_SHIFT_RELEASE: u8 = 0xAA;

const KEYMAP: [char; 128] = {
    let mut map = ['\0'; 128];
    map[0x01] = '\x1B';
    map[0x02] = '1';
    map[0x03] = '2';
    map[0x04] = '3';
    map[0x05] = '4';
    map[0x06] = '5';
    map[0x07] = '6';
    map[0x08] = '7';
    map[0x09] = '8';
    map[0x0a] = '9';
    map[0x0b] = '0';
    map[0x0c] = '-';
    map[0x0d] = '=';
    map[0x0e] = '\x08';
    map[0x0f] = '\t';
    map[0x10] = 'q';
    map[0x11] = 'w';
    map[0x12] = 'e';
    map[0x13] = 'r';
    map[0x14] = 't';
    map[0x15] = 'y';
    map[0x16] = 'u';
    map[0x17] = 'i';
    map[0x18] = 'o';
    map[0x19] = 'p';
    map[0x1a] = '[';
    map[0x1b] = ']';
    map[0x1c] = '\n';
    map[0x1e] = 'a';
    map[0x1f] = 's';
    map[0x20] = 'd';
    map[0x21] = 'f';
    map[0x22] = 'g';
    map[0x23] = 'h';
    map[0x24] = 'j';
    map[0x25] = 'k';
    map[0x26] = 'l';
    map[0x27] = ';';
    map[0x28] = '\'';
    map[0x29] = '`';
    map[0x2b] = '\\';
    map[0x2c] = 'z';
    map[0x2d] = 'x';
    map[0x2e] = 'c';
    map[0x2f] = 'v';
    map[0x30] = 'b';
    map[0x31] = 'n';
    map[0x32] = 'm';
    map[0x33] = ',';
    map[0x34] = '.';
    map[0x35] = '/';
    map[0x39] = ' ';
    map
};

const KEYMAP_SHIFT_SYMBOLS: [char; 128] = {
    let mut map = ['\0'; 128];

    map[0x02] = '!';
    map[0x03] = '@';
    map[0x04] = '#';
    map[0x05] = '$';
    map[0x06] = '%';
    map[0x07] = '^';
    map[0x08] = '&';
    map[0x09] = '*';
    map[0x0a] = '(';
    map[0x0b] = ')';
    map[0x0c] = '_';
    map[0x0d] = '+';

    map[0x1a] = '{';
    map[0x1b] = '}';
    map[0x27] = ':';
    map[0x28] = '"';
    map[0x29] = '~';
    map[0x2b] = '|';
    map[0x33] = '<';
    map[0x34] = '>';
    map[0x35] = '?';

    map
};

pub struct Keymap {
    shift_pressed: bool,
}

impl Keymap {
    pub fn new() -> Self {
        Keymap {
            shift_pressed: false,
        }
    }

    fn shift_pressed(&mut self) {
        self.shift_pressed = true;
    }

    fn shift_released(&mut self) {
        self.shift_pressed = false
    }

    fn shift_convert(&self, scancode: u8) -> char {
        match scancode {
            0..=127 => {
                let simbolo_special = KEYMAP_SHIFT_SYMBOLS[scancode as usize];
                if simbolo_special != '\0' {
                    simbolo_special
                } else {
                    KEYMAP[scancode as usize].to_ascii_uppercase()
                }
            }
            _ => '\0',
        }
    }

    pub fn convert(&mut self, scancode: u8) -> char {
        if scancode == LEFT_SHIFT_PRESS {
            self.shift_pressed();
            '\0'
        } else if scancode == LEFT_SHIFT_RELEASE {
            self.shift_released();
            '\0'
        } else {
            match scancode {
                0..=127 => {
                    let c = KEYMAP[scancode as usize];
                    if self.shift_pressed {
                        self.shift_convert(scancode)
                    } else {
                        c
                    }
                }
                _ => '\0',
            }
        }
    }
}

lazy_static! {
    pub static ref KEYMAPDRIVER: Mutex<Keymap> = Mutex::new(Keymap::new());
}
