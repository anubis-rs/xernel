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
}

pub struct SpinlockIRQGuard<'a, T: 'a> {
    guard: MutexGuard<'a, T>,
    _held_irq: HeldIRQ,
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

// TODO: Differ in target platform, so this function can be used across multiple architectures
#[inline(always)]
pub fn interrupts_enabled() -> bool {
    unsafe {
        let flags: usize;
        asm!("pushfq; pop {}", out(reg) flags, options(nomem, preserves_flags));
        (flags & 0x0200) != 0
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
        asm!("cli", options(nomem, nostack));
    }
    compiler_fence(Ordering::SeqCst);
}

#[inline(always)]
pub fn enable_interrupts() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}
