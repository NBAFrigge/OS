use core::sync::atomic::{AtomicU64, Ordering};

static TSC_PER_US: AtomicU64 = AtomicU64::new(0);
static TSC_BASE: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn set_calibration(cycles_per_us: u64) {
    TSC_PER_US.store(cycles_per_us.max(1), Ordering::Relaxed);
    TSC_BASE.store(rdtsc(), Ordering::Relaxed);
}

pub fn is_calibrated() -> bool {
    TSC_PER_US.load(Ordering::Relaxed) > 0
}

#[inline]
pub fn now_us() -> u64 {
    let per_us = TSC_PER_US.load(Ordering::Relaxed);
    if per_us == 0 {
        return 0;
    }
    rdtsc().wrapping_sub(TSC_BASE.load(Ordering::Relaxed)) / per_us
}
