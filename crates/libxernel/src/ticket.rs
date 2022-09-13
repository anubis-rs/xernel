use core::{sync::atomic::AtomicUsize, cell::UnsafeCell, sync::atomic::Ordering, ops::{Deref, DerefMut}};

pub struct TicketMutex<T> {
    next_ticket: AtomicUsize,
    next_serving: AtomicUsize,
    data: UnsafeCell<T>,
}

pub struct TicketMutexGuard<'a, T: 'a> {
    ticket: usize,
    mutex: &'a TicketMutex<T>,
}

unsafe impl<T> Send for TicketMutex<T> {}
unsafe impl<T> Sync for TicketMutex<T> {}

impl<T> TicketMutex<T> {

    pub const fn new(data: T) -> Self {
        TicketMutex {
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> TicketMutexGuard<'_, T> {

        let ticket = self.next_ticket.fetch_add(1, Ordering::Acquire);

        while self.next_serving.load(Ordering::Acquire) != ticket {
            core::hint::spin_loop();
        }

        TicketMutexGuard {
            mutex: &self,
            ticket: ticket,
        }

    }

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