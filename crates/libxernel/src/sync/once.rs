use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::Deref,
    panic,
    sync::atomic::{AtomicBool, Ordering},
};

/// Type which represents a value which gets set exactly once
///
/// This type will allow to be set once, then never again.
/// Made for values which are only available at runtime and is used in a [`static`] context
pub struct Once<T> {
    /// Determines if the value is set or if it's uninitialized
    is_set: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Once<T> {
    /// Creates a new uninitialized Once object
    pub const fn new() -> Self {
        Self {
            is_set: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Sets the value
    ///
    /// # Panics
    /// Panics if the value is already set
    pub fn set_once(&self, val: T) {
        // Checks if the value is already set
        if !self.is_set.load(Ordering::Acquire) {
            unsafe {
                // Write data to UnsafeCell
                (*self.data.get()).as_mut_ptr().write(val);
            }
            // Set is_set value to true
            self.is_set.store(true, Ordering::Release);
        } else {
            // If already set panic!
            panic!("Value already set");
        }
    }

    /// Returns `true` if some [`set_once()`](struct.Once.html#methods.set_once) has completed successfully
    pub fn is_completed(&self) -> bool {
        self.is_set.load(Ordering::Relaxed)
    }
}

unsafe impl<T> Send for Once<T> {}
unsafe impl<T> Sync for Once<T> {}

impl<T> Deref for Once<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Check if the value is_set
        if self.is_set.load(Ordering::Acquire) {
            // Return a reference if set
            unsafe { &*(*self.data.get()).as_ptr() }
        } else {
            // panic! if uninitialized
            panic!("Value not set");
        }
    }
}
