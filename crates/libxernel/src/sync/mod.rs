pub use self::once::Once;
pub use self::rwlock::{ReadGuard, RwLock, WriteGuard};
pub use self::spin::{MutexGuard, Spinlock};
pub use self::ticket::{TicketMutex, TicketMutexGuard};

mod once;
mod rwlock;
mod spin;
mod ticket;

#[cfg(feature = "kernel")]
mod spinirq;
#[cfg(feature = "kernel")]
pub use self::spinirq::{SpinlockIRQ, SpinlockIRQGuard};
