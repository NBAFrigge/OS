use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

const COM1: u16 = 0x3F8;

lazy_static! {
    pub static ref SERIAL_LOGGER: Mutex<SerialPort> = {
        let mut port = unsafe { SerialPort::new(COM1) };
        port.init();
        Mutex::new(port)
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL_LOGGER
        .lock()
        .write_fmt(args)
        .expect("Serial print failed");
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::serialprinter::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    };
}
