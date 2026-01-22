pub use self::once::Once;
pub use self::rwlock::{ReadGuard, RwLock, WriteGuard};
pub use self::spin::{Spinlock, SpinlockGuard};

mod once;
mod rwlock;
mod spin;

#[cfg(feature = "kernel")]
mod spinirq;
#[cfg(feature = "kernel")]
pub use self::spinirq::{SpinlockIRQ, SpinlockIRQGuard};
