use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// Simple data locking structure using a spin loop.
///
/// This spinlock will block threads waiting for the lock to become available.
/// Accessing the data is only possible through the RAII guards returned from [`Spinlock::lock`] and [`Spinlock::try_lock`], since they guarantee you are the owner of the lock.
pub struct Spinlock<T> {
    /// Atomic variable which is used to determine if the Spinlock is locked or not
    is_locked: AtomicBool,
    /// The data itself
    data: UnsafeCell<T>,
}

/// Spinlock RAII wrapper type for safe release of lock
///
/// When acquiring a lock through [`Spinlock::lock`] or [`Spinlock::try_lock`], a MutexGuard gets returned which is a wrapper over the mutex itself.
/// This type is used for releasing the spinlock when the value goes out of scope, so you don't have to think of unlocking yourself.
pub struct MutexGuard<'a, T: 'a> {
    mutex: &'a Spinlock<T>,
}

unsafe impl<T> Send for Spinlock<T> {}
unsafe impl<T> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Creates an unlocked and initialized spinlock
    pub const fn new(data: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires a lock for this spinlock and returns a RAII guard
    ///
    /// It tries to acquire the lock, if it's already locked the thread enters a so-called spin loop
    /// When the value of the underlying atomic boolean changes, it tries again to acquire the lock but no guarantee given
    /// that it will be given the lock.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        loop {
            if !self.is_locked.swap(true, Ordering::Acquire) {
                return MutexGuard { mutex: self };
            }

            while self.is_locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Tries one time to acquire the lock
    ///
    /// Simply a try if the lock is free, if not [`None`] returned, else a MutexGuard wrapped in an option
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if !self.is_locked.swap(true, Ordering::AcqRel) {
            // is_locked was false and now we have atomically swapped it to true,
            // so no one else has access to this data.
            return Some(MutexGuard { mutex: self });
        }
        None
    }

    /// Unlocking a spinlock
    ///
    /// With the drop approach the lock only gets released when the MutexGuard value goes out of scope.
    /// It is possible to earlier drop the value with `drop(guard);` but it looks like unclean programming.
    /// This associated function is no different to `drop()` but when reading the code it is much clearer what is happening.
    pub fn unlock(_guard: MutexGuard<'_, T>) {}
}

impl<'a, T: 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        // Releasing the lock
        self.mutex.is_locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}
