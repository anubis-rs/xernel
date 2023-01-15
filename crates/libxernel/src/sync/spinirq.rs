use crate::sync::Spinlock;
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{compiler_fence, Ordering};

use super::MutexGuard;

pub struct HeldIRQ(bool);

pub struct SpinlockIRQ<T> {
    lock: Spinlock<T>,
}

impl<T> SpinlockIRQ<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: Spinlock::new(data),
        }
    }

    pub fn lock(&self) -> SpinlockIRQGuard<T> {
        let inner_lock = self.lock.lock();

        SpinlockIRQGuard {
            guard: inner_lock,
            _held_irq: hold_interrupts(),
        }
    }

    pub fn unlock(_guard: SpinlockIRQGuard<'_, T>) {}
}

pub struct SpinlockIRQGuard<'a, T: 'a> {
    guard: MutexGuard<'a, T>,
    _held_irq: HeldIRQ,
}

impl<T> SpinlockIRQGuard<'_, T> {
    pub fn unlock(self) {}
}

impl<'a, T> Deref for SpinlockIRQGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(self.guard)
    }
}

impl<'a, T> DerefMut for SpinlockIRQGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut (self.guard)
    }
}

impl Drop for HeldIRQ {
    fn drop(&mut self) {
        if self.0 {
            enable_interrupts();
        }
    }
}

#[inline(always)]
pub fn interrupts_enabled() -> bool {
    if cfg!(target_arch = "x86_64") {
        unsafe {
            let flags: usize;
            asm!("pushfq; pop {}", out(reg) flags, options(nomem, preserves_flags));
            (flags & 0x0200) != 0
        }
    } else {
        unimplemented!("Interrupts enabled not implemented for this architecture");
    }
}

pub fn hold_interrupts() -> HeldIRQ {
    let enabled = interrupts_enabled();
    let retval = HeldIRQ(enabled);
    disable_interrupts();
    retval
}

#[inline(always)]
pub fn disable_interrupts() {
    unsafe {
        if cfg!(target_arch = "x86_64") {
            asm!("cli", options(nomem, nostack));
        } else {
            unimplemented!("Disable interrupts not implemented for this architecture");
        }
    }
    compiler_fence(Ordering::SeqCst);
}

#[inline(always)]
pub fn enable_interrupts() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        if cfg!(target_arch = "x86_64") {
            asm!("sti", options(nomem, nostack));
        } else {
            unimplemented!("Enable interrupts not implemented for this architecture");
        }
    }
}
