use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::Deref,
    panic,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Once<T> {
    is_set: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Once<T> {
    pub const fn new() -> Self {
        Self {
            is_set: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn set_once(&self, val: T) {
        if !self.is_set.load(Ordering::Acquire) {
            unsafe {
                (*self.data.get()).as_mut_ptr().write(val);
            }
            self.is_set.store(true, Ordering::Release);
        } else {
            panic!("Value already set");
        }
    }
}

unsafe impl<T> Send for Once<T> {}
unsafe impl<T> Sync for Once<T> {}

impl<T> Deref for Once<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.data.get()).as_ptr() }
    }
}
