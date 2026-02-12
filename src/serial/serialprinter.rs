use spin::Mutex;
use lazy_static::lazy_static;
use uart_16550::SerialPort;

const COM1: u16 = 0x3F8;

lazy_static! {
    pub static ref SERIAL_LOGGER: Mutex<SerialPort> = {
        let mut port = unsafe { SerialPort::new(COM1) };
        port.init();
        Mutex::new(port)
    };
}

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {
        use core::fmt::Write;
        let mut port = $crate::serial::serialprinter::SERIAL_LOGGER.lock();
        writeln!(port, $($arg)*).expect("Serial print failed");
    };
}