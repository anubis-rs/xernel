use core::mem::MaybeUninit;
use core::ptr;

struct Ringbuffer<T, const N: usize> { 
    buffer: [MaybeUninit<T>; N],
    read: usize,
    write: usize,
    size: usize
}

impl <T, const N: usize> Ringbuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::<T>::zeroed() }; N],
            read: 0,
            write: 0,
            size: 0,
        }
    }
    
    pub fn push(&mut self, value: T) {
        if N == 0 {
            return;
        }
        
        if self.size >= N {
            unsafe {
               ptr::drop_in_place(self.buffer[self.write].as_mut_ptr()); 
            }
            
            self.buffer[self.write].write(value);
            self.wrap_inc_write();
            
        } else {
            self.buffer[self.write].write(value);
            self.wrap_inc_write();
            self.inc_size();
        }
    }
    
    pub fn pop(&mut self)  -> Option<T> {
        if N == 0 {
            return None;
        }
        
        if self.is_empty() {
            return None;
        }
        
        let ret = unsafe {
            self.buffer[self.read].assume_init_read()
        };
        
        self.wrap_inc_read();
        self.dec_size();
        
        Some(ret)
    }
    
    pub fn skip(&mut self) {
        
        if N == 0 {
            return;
        }
        
        if self.is_empty() {
            return;
        }
        
        unsafe {
            ptr::drop_in_place(self.buffer[self.read].as_mut_ptr());
        }
        
        self.wrap_inc_read();
        self.dec_size();
    }
    
    pub fn peek(&mut self) -> Option<&T> {
        if N == 0 {
            return None;
        }
        
        if self.is_empty() {
            return None;
        }
        
        let ret = unsafe {
            self.buffer[self.read].assume_init_ref()  
        };
        
        Some(ret)
    }
    
    pub fn size(&self) -> usize {
        self.size
    }
    
    pub fn capacity(&self) -> usize {
        N
    }
    
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    
    pub fn is_full(&self) -> bool {
        self.size == N
    }
    
    fn wrap_inc_read(&mut self) {
        self.read = (self.read + 1) % N;
    }
    
    fn wrap_inc_write(&mut self) {
        self.write = (self.write + 1) % N;
    }
    
    fn inc_size(&mut self) {
        self.size += 1;
    }
    
    fn dec_size(&mut self) {
        self.size -= 1;
    }
}

impl<T, const N: usize> Drop for Ringbuffer<T, N> {
    fn drop(&mut self) {
        while let Some(value) = self.peek() {
            self.skip()
       }
    }
}
