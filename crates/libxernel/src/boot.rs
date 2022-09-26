use core::{ops::{Deref, DerefMut}, cell::UnsafeCell, mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering}, panic};

pub struct InitAtBoot<T> {
    is_set: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>
}

impl<T> InitAtBoot<T> {

    pub const fn new() -> Self {
        Self {
            is_set: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit())
        }
    }

    pub fn set_once(&self, val: T) {
        if !self.is_set.load(Ordering::Acquire) {
            unsafe { (*self.data.get()).as_mut_ptr().write(val); }
            self.is_set.store(true, Ordering::Release);
        } else {
            panic!("Value already set");
        }
    }

}

impl<T> Deref for InitAtBoot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.data.get()).as_ptr() }
    }
}

impl<T> DerefMut for InitAtBoot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.data.get()).as_mut_ptr() }
    }
}
