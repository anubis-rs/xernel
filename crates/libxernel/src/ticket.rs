use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
    sync::atomic::Ordering,
};

/// Data locking structure with ticket system
///
/// This Mutex uses a spinlock to block the threads while waiting for the lock to become available.
/// When trying to lock then the thread gets a ticket, when the lock is released it looks which ticket is next in line and the thread with that ticket gets the lock
pub struct TicketMutex<T> {
    /// Value used for giving out new tickets to threads
    next_ticket: AtomicUsize,
    /// Next ticket which acquires the lock when lock is free
    next_serving: AtomicUsize,
    /// Data wrapped in [`UnsafeCell`] for interior mutability
    data: UnsafeCell<T>,
}

/// RAII wrapper type for safe release of lock
///
/// When acquiring a lock through  [`TicketMutex::lock`] or [`TicketMutex::try_lock`], a [`TicketMutexGuard`] gets returned which is a wrapper over the mutex itself.
/// This type is used for releasing the ticket mutex when the value goes out of scope, so you don't have to think of unlocking yourself.
pub struct TicketMutexGuard<'a, T: 'a> {
    ticket: usize,
    mutex: &'a TicketMutex<T>,
}

unsafe impl<T> Send for TicketMutex<T> {}
unsafe impl<T> Sync for TicketMutex<T> {}

impl<T> TicketMutex<T> {
    /// Creates an unlocked and initialized `TicketMutex`
    pub const fn new(data: T) -> Self {
        TicketMutex {
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires a lock for this ticket mutex and returns a RAII guard
    ///
    /// It tries to acquire the lock, if it's already locked the thread enters a so-called spin loop.
    /// When the next_serving value changes (the currently held lock goes out of scope or gets manually unlocked) the Mutex checks if it has the next_serving ticket. If that's not the case it goes spinning again, else it acquires the lock and returns the wrapper type
    pub fn lock(&self) -> TicketMutexGuard<'_, T> {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        while self.next_serving.load(Ordering::Acquire) != ticket {
            core::hint::spin_loop();
        }

        TicketMutexGuard {
            mutex: self,
            ticket,
        }
    }

    /// Tries one time to acquire the lock
    ///
    /// Simply a try if the lock is free, if not [`None`] returned, else a [`TicketMutexGuard`] wrapped in an option
    pub fn try_lock(&self) -> Option<TicketMutexGuard<'_, T>> {
        let ticket = self
            .next_ticket
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ticket| {
                if self.next_serving.load(Ordering::Acquire) == ticket {
                    Some(ticket + 1)
                } else {
                    None
                }
            });

        ticket.ok().map(|ticket| TicketMutexGuard {
            ticket,
            mutex: self,
        })
    }

    /// Unlocking a spinlock
    ///
    /// With the drop approach the lock only gets released when the [`TicketMutexGuard`] value goes out of scope.
    /// It is possible to earlier drop the value with `drop(guard);` but it looks like unclean programming.
    /// This associated function is no different to [`drop`] but when reading the code it is much clearer what is happening.
    pub fn unlock(_guard: TicketMutexGuard<'_, T>) {}
}

impl<'a, T: 'a> Drop for TicketMutexGuard<'a, T> {
    fn drop(&mut self) {
        let new_ticket = self.ticket + 1;
        self.mutex.next_serving.store(new_ticket, Ordering::Release);
    }
}

impl<'a, T> Deref for TicketMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for TicketMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> TicketMutexGuard<'a, T> {
    pub fn ticket(&self) -> usize {
        self.ticket
    }
}
