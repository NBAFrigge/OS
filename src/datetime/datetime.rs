use crate::{datetime::rtcdatetime::read_rtc, idt::interrupt::TICKS};
use core::fmt;

pub struct Datetime {
    timestmap: u64,
}

pub struct FormattedTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Datetime {
    pub fn new() -> Self {
        unsafe {
            Datetime {
                timestmap: read_rtc(),
            }
        }
    }

    pub fn now(&self) -> u64 {
        self.timestmap
            + TICKS.load(core::sync::atomic::Ordering::Relaxed) / 1000
    }

    pub fn date(&self) -> FormattedTime {
        let timestamp = self.now();
        let second = (timestamp % 60) as u8;
        let minute = ((timestamp / 60) % 60) as u8;
        let hour = ((timestamp / 3600) % 24) as u8;

        let mut days = timestamp / 86400;
        let mut year = 1970;

        loop {
            let is_leap =
                (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
            let days_in_year = if is_leap { 366 } else { 365 };
            if days < days_in_year {
                break;
            }
            days -= days_in_year;
            year += 1;
        }

        let is_leap = (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
        let days_in_month = [
            31,
            if is_leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];

        let mut month = 1;
        for &m_days in days_in_month.iter() {
            if days < m_days as u64 {
                break;
            }
            days -= m_days as u64;
            month += 1;
        }

        FormattedTime {
            year: year as u16,
            month: month as u8,
            day: (days + 1) as u8,
            hour,
            minute,
            second,
        }
    }
}

impl fmt::Display for FormattedTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}/{:02}/{} {:02}:{:02}:{:02}",
            self.day,
            self.month,
            self.year,
            self.hour,
            self.minute,
            self.second
        )
    }
}
