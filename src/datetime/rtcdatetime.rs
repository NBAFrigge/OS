use x86_64::instructions::port::Port;

unsafe fn is_updating() -> bool {
    let mut addr_port = Port::<u8>::new(0x70);
    let mut data_port = Port::<u8>::new(0x71);

    addr_port.write(0x0A);
    (data_port.read() & 0x80) != 0
}

unsafe fn read_register(reg: u8) -> u8 {
    let mut addr_port = Port::<u8>::new(0x70);
    let mut data_port = Port::<u8>::new(0x71);

    addr_port.write(reg);
    data_port.read()
}

pub unsafe fn read_rtc() -> u64 {
    while is_updating() {}

    let mut second = read_register(0x00);
    let mut minute = read_register(0x02);
    let mut hour = read_register(0x04);
    let mut day = read_register(0x07);
    let mut month = read_register(0x08);
    let mut year = read_register(0x09) as u16;

    let status_b = read_register(0x0B);

    if (status_b & 0x04) == 0 {
        second = (second & 0x0F) + ((second / 16) * 10);
        minute = (minute & 0x0F) + ((minute / 16) * 10);
        hour = ((hour & 0x0F) + (((hour & 0x70) / 16) * 10)) | (hour & 0x80);
        day = (day & 0x0F) + ((day / 16) * 10);
        month = (month & 0x0F) + ((month / 16) * 10);
        year = (year & 0x0F) + ((year / 16) * 10);
    }

    let full_year = 2000 + year as u64;

    let mut total_seconds: u64 = (full_year - 1970) * 31536000;
    total_seconds += ((full_year - 1969) / 4) * 86400;

    let days_per_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for i in 1..month as usize {
        total_seconds += days_per_month[i] * 86400;
    }

    if month > 2 && full_year % 4 == 0 {
        total_seconds += 86400;
    }

    total_seconds += (day as u64 - 1) * 86400;
    total_seconds += (hour as u64) * 3600;
    total_seconds += (minute as u64) * 60;
    total_seconds += second as u64;

    total_seconds
}
