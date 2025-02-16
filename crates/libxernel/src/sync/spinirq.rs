use crate::sync::Spinlock;
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{compiler_fence, Ordering};

use super::SpinlockGuard;

/// A handle for interrupt state
pub struct HeldIRQ(bool);

/// Spinlock which disables interrupts when taking the lock
pub struct SpinlockIRQ<T> {
    lock: Spinlock<T>,
}

impl<T> SpinlockIRQ<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: Spinlock::new(data),
        }
    }

    /// Calls the lock of the inner [`Spinlock`] and freezes the interrupts
    pub fn lock(&self) -> SpinlockIRQGuard<T> {
        let inner_lock = self.lock.lock();

        SpinlockIRQGuard {
            guard: inner_lock,
            _held_irq: hold_interrupts(),
        }
    }

    /// Unlock the underlying spinlock
    ///
    /// If needed to release the lock before the Guard gets dropped you may use this function
    pub fn unlock(_guard: SpinlockIRQGuard<'_, T>) {}
}

/// Wrapper Type over SpinlockGuard and HeldIRQ
pub struct SpinlockIRQGuard<'a, T: 'a> {
    guard: SpinlockGuard<'a, T>,
    _held_irq: HeldIRQ,
}

impl<T> SpinlockIRQGuard<'_, T> {
    /// Unlock the underlying spinlock
    ///
    /// If needed to release the lock before the Guard gets dropped you may use this function
    pub fn unlock(self) {}
}

impl<T> Deref for SpinlockIRQGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(self.guard)
    }
}

impl<T> DerefMut for SpinlockIRQGuard<'_, T> {
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

/// Gets the interrupt state
///
/// Returns a bool if interrupts are currently enabled or not.
/// Is used when dropping the SpinlockIRQGuard to get back to the old interrupt state.
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

/// Returns a HeldIRQ object with the current interrupt state
///
/// Gets the current interrupt state and creates a HeldIRQ object
/// It then disables the interrupts, even if they are already disabled and returns the HeldIRQ object.
pub fn hold_interrupts() -> HeldIRQ {
    let enabled = interrupts_enabled();
    let retval = HeldIRQ(enabled);
    disable_interrupts();
    retval
}

/// Disables interrupts across multiple architectures
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

/// Enables interrupts across multiple architectures
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
