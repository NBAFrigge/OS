use core::{
    fmt,
    sync::atomic::{AtomicU8, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" => Some(LogLevel::Error),
            "warn" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            "trace" => Some(LogLevel::Trace),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        }
    }
}

pub struct KernelLogger {
    target_level: AtomicU8,
}

impl KernelLogger {
    pub fn log(&self, level: LogLevel, target: &str, args: fmt::Arguments) {
        let current_level = self.target_level.load(Ordering::Relaxed);
        if level as u8 <= current_level {
            let (level_str, color_code) = match level {
                LogLevel::Error => ("ERROR", "\x1b[31m"),
                LogLevel::Warn => ("WARN ", "\x1b[33m"),
                LogLevel::Info => ("INFO ", "\x1b[32m"),
                LogLevel::Debug => ("DEBUG", "\x1b[36m"),
                LogLevel::Trace => ("TRACE", "\x1b[90m"),
            };

            serial_println!(
                "{}[{}]\x1b[0m [{}] {}",
                color_code,
                level_str,
                target,
                args
            );
        }
    }

    pub fn set_log_level(&self, level: LogLevel) {
        self.target_level.store(level as u8, Ordering::Relaxed);
    }

    pub fn get_log_level(&self) -> LogLevel {
        match self.target_level.load(Ordering::Relaxed) {
            0 => LogLevel::Error,
            1 => LogLevel::Warn,
            2 => LogLevel::Info,
            3 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }
}

pub static LOGGER: KernelLogger = KernelLogger {
    target_level: AtomicU8::new(LogLevel::Info as u8),
};

#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => {
        $crate::logger::logger::LOGGER.log(
            $crate::logger::logger::LogLevel::Error,
            module_path!(),
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => {
        $crate::logger::logger::LOGGER.log(
            $crate::logger::logger::LogLevel::Warn,
            module_path!(),
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => {
        $crate::logger::logger::LOGGER.log(
            $crate::logger::logger::LogLevel::Info,
            module_path!(),
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => {
        $crate::logger::logger::LOGGER.log(
            $crate::logger::logger::LogLevel::Debug,
            module_path!(),
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! ktrace {
    ($($arg:tt)*) => {
        $crate::logger::logger::LOGGER.log(
            $crate::logger::logger::LogLevel::Trace,
            module_path!(),
            format_args!($($arg)*)
        )
    };
}
