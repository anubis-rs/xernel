use core::ops::{Deref, DerefMut};

pub enum InitAtBoot<T> {
    Initialized(T),
    Uninitialized,
  }
  
  
impl<T> Deref for InitAtBoot<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            InitAtBoot::Initialized(ref x) => x,
            InitAtBoot::Uninitialized => {
                #[cfg(debug_assertions)]
                panic!("tried to access boot resource that is not initialized");
                #[cfg(not(debug_assertions))]
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }
}
