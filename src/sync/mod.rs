use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

pub struct IrqMutex<T> {
    inner: spin::Mutex<T>,
}

pub struct IrqMutexGuard<'a, T> {
    guard: ManuallyDrop<spin::MutexGuard<'a, T>>,
    restore: bool,
}

impl<T> IrqMutex<T> {
    pub const fn new(val: T) -> Self {
        IrqMutex { inner: spin::Mutex::new(val) }
    }

    pub fn lock(&self) -> IrqMutexGuard<T> {
        let restore = x86_64::instructions::interrupts::are_enabled();
        x86_64::instructions::interrupts::disable();
        IrqMutexGuard {
            guard: ManuallyDrop::new(self.inner.lock()),
            restore,
        }
    }
}

unsafe impl<T: Send> Send for IrqMutex<T> {}
unsafe impl<T: Send> Sync for IrqMutex<T> {}

impl<'a, T> Drop for IrqMutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.guard) };
        if self.restore {
            x86_64::instructions::interrupts::enable();
        }
    }
}

impl<'a, T> Deref for IrqMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<'a, T> DerefMut for IrqMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}
