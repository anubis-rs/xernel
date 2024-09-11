use core::{
    arch::asm,
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ipl::{raise_ipl, splx, IPL},
    on_drop::OnDrop,
};

/// Simple data locking structure using a spin loop.
///
/// This spinlock will block threads waiting for the lock to become available.
/// Accessing the data is only possible through the RAII guards returned from [`Spinlock::lock`] and [`Spinlock::try_lock`], since they guarantee you are the owner of the lock.
pub struct Spinlock<T: ?Sized> {
    /// Atomic variable which is used to determine if the Spinlock is locked or not
    is_locked: AtomicBool,
    /// The data itself
    data: UnsafeCell<T>,
}

/// Spinlock RAII wrapper type for safe release of lock
///
/// When acquiring a lock through [`Spinlock::lock`] or [`Spinlock::try_lock`], a SpinlockGuard gets returned which is a wrapper over the mutex itself.
/// This type is used for releasing the spinlock when the value goes out of scope, so you don't have to think of unlocking yourself.
pub struct SpinlockGuard<'a, T: ?Sized + 'a> {
    lock: &'a Spinlock<T>,
}

unsafe impl<T: ?Sized> Send for Spinlock<T> {}
unsafe impl<T: ?Sized> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Creates an unlocked and initialized spinlock
    pub const fn new(data: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    /// Acquires a lock for this spinlock and returns a RAII guard
    ///
    /// It tries to acquire the lock, if it's already locked the thread enters a so-called spin loop
    /// When the value of the underlying atomic boolean changes, it tries again to acquire the lock but no guarantee given
    /// that it will be given the lock.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        loop {
            if !self.is_locked.swap(true, Ordering::Acquire) {
                return SpinlockGuard { lock: self };
            }

            while self.is_locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Tries one time to acquire the lock
    ///
    /// Simply a try if the lock is free, if not [`None`] returned, else a [`SpinlockGuard`] wrapped in an option
    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        if !self.is_locked.swap(true, Ordering::AcqRel) {
            // is_locked was false and now we have atomically swapped it to true,
            // so no one else has access to this data.
            return Some(SpinlockGuard { lock: self });
        }
        None
    }

    pub fn with_lock<F, U>(&self, function: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut lock = self.lock();
        function(&mut *lock)
    }

    pub fn aquire(&self) -> OnDrop<SpinlockGuard<'_, T>, impl FnOnce()> {
        let ipl = raise_ipl(IPL::DPC);
        let callback = move || splx(ipl);
        OnDrop::new(self.lock(), callback)
    }

    //pub fn aquire_at(&self, ipl: IPL) -> OnDrop<T, F> {}

    /// Unlocking a spinlock
    ///
    /// With the drop approach the lock only gets released when the [`SpinlockGuard`] value goes out of scope.
    /// It is possible to earlier drop the value with `drop(guard);` but it looks like unclean programming.
    /// This associated function is no different to [`drop`] but when reading the code it is much clearer what is happening.
    pub fn unlock(_guard: SpinlockGuard<'_, T>) {}
}

impl<T: ?Sized> SpinlockGuard<'_, T> {
    /// Unlocking a spinlock
    ///
    /// Sometimes it is nice to be able to unlock a lock when you want to.
    /// Normally a Spinlock in Rust would only unlock when the corresponding Guard would be dropped.
    /// In special cases, like the [`Scheduler`], we even need the lock to be released before the function end, since we would wind up in a dead lock on the next timer interrupt.
    /// Semantically there is no difference between this method and [`Spinlock::unlock`](struct.Spinlock.html#method.unlock)
    pub fn unlock(self) {}
}

impl<'a, T: ?Sized> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        // Releasing the lock
        self.lock.is_locked.store(false, Ordering::Release);
    }
}

impl<'a, T: ?Sized> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

#[inline]
fn write_cr8(ipl: IPL) {
    unsafe {
        asm!("mov cr8, {}", in(reg) ipl as u64, options(nomem, nostack, preserves_flags));
    }
}
