use core::mem::ManuallyDrop;

use core::ops::{Deref, DerefMut};

use crate::sync::SpinlockGuard;

pub struct OnDrop<T, F>
where
    F: FnOnce(),
{
    value: ManuallyDrop<T>,
    callback: ManuallyDrop<F>,
}

impl<T, F> OnDrop<T, F>
where
    F: FnOnce(),
{
    pub fn new(value: T, callback: F) -> Self {
        Self {
            value: ManuallyDrop::new(value),
            callback: ManuallyDrop::new(callback),
        }
    }
}

impl<T, F> Drop for OnDrop<T, F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::<T>::drop(&mut self.value);
            (ManuallyDrop::<F>::take(&mut self.callback))();
        }
    }
}

impl<'a, T, F> OnDrop<SpinlockGuard<'a, T>, F>
where
    F: FnOnce(),
{
    pub fn unlock(self) {}
}

impl<T, F> Deref for OnDrop<T, F>
where
    F: FnOnce(),
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, F> DerefMut for OnDrop<T, F>
where
    F: FnOnce(),
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
