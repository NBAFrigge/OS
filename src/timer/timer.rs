#[macro_export]
macro_rules! msleep {
    ($ms:expr) => {
        $crate::timer::timer::_msleep($ms)
    };
}

pub fn _msleep(ms: u64) {
    let current_ticks = crate::idt::interrupt::TICKS
        .load(core::sync::atomic::Ordering::Relaxed);
    let release_timer = current_ticks + ms;

    while crate::idt::interrupt::TICKS
        .load(core::sync::atomic::Ordering::Relaxed)
        < release_timer
    {
        x86_64::instructions::hlt();
    }
}
