use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::pin::Pin;
use core::cell::{Cell, UnsafeCell};
use core::sync::atomic::{AtomicUsize, Ordering};
use libxernel::sync::{Once, RwLock};

use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::{KERNEL_GS_BASE, rdmsr, wrmsr};
use crate::sched::thread::Thread;

static CPU_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub static CPU_COUNT: Once<usize> = Once::new();

pub struct PerCpu<T> {
    data: UnsafeCell<Vec<T>>,
}

unsafe impl<T> Send for PerCpu<T> {}
unsafe impl<T> Sync for PerCpu<T> {}

impl<T> PerCpu<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(Vec::<T>::new()),
        }
    }

    pub fn init(&self, init_fn: fn() -> T) {
        assert_eq!(*CPU_COUNT, CPU_ID_COUNTER.load(Ordering::SeqCst));

        let vec = unsafe { &mut *self.data.get() };

        for _ in 0..*CPU_COUNT {
            vec.push(init_fn());
        }
    }

    pub fn wait_until_initialized(&self) {
        loop {
            let vec = unsafe { &*self.data.get() };

            if vec.len() == *CPU_COUNT {
                break;
            }

            core::hint::spin_loop();
        }
    }

    fn check_initialized(&self) {
        let vec = unsafe { &*self.data.get() };

        assert_eq!(vec.len(), *CPU_COUNT);
    }

    pub fn get(&self) -> &T {
        self.check_initialized();

        let cpu_id = current_cpu().cpu_id;
        let vec = unsafe { &mut *self.data.get() };
        &vec[cpu_id]
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        self.check_initialized();

        let cpu_id = current_cpu().cpu_id;
        let vec = unsafe { &mut *self.data.get() };
        &mut vec[cpu_id]
    }

    pub unsafe fn get_index(&self, index: usize) -> &T {
        self.check_initialized();

        let vec = unsafe { &mut *self.data.get() };
        &vec[index]
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_index_mut(&self, index: usize) -> &mut T {
        self.check_initialized();

        let vec = unsafe { &mut *self.data.get() };
        &mut vec[index]
    }

    pub unsafe fn get_all(&self) -> &Vec<T> {
        self.check_initialized();

        &*self.data.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_all_mut(&self) -> &mut Vec<T> {
        self.check_initialized();

        &mut *self.data.get()
    }
}

#[repr(C)]
pub struct Cpu {
    // NOTE: don't move these variables as we need to access them from assembly
    user_space_stack: usize,
    pub kernel_stack: Cell<usize>,

    cpu_id: usize,
    lapic_id: u32,
    pub run_queue: RwLock<VecDeque<Arc<Thread>>>,
    pub wait_queue:  RwLock<VecDeque<Arc<Thread>>>,
    pub current_thread:  RwLock<Option<Arc<Thread>>>,
    pub idle_thread: Arc<Thread>,
}

pub fn register_cpu() {
    let cpu_id = CPU_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let lapic_id = APIC.lapic_id();

    let cpu_data = Box::leak(Box::new(Cpu {
        user_space_stack: 0,
        kernel_stack: Cell::new(0),
        cpu_id,
        lapic_id: 0,
        run_queue: RwLock::new(VecDeque::new()),
        wait_queue: RwLock::new(VecDeque::new()),
        current_thread: RwLock::new(None),
        idle_thread: Arc::new(Thread::idle_thread()),
    }));

    // use KERNEL_GS_BASE to store the cpu_data
    unsafe {
        wrmsr(KERNEL_GS_BASE, (cpu_data as *const Cpu).expose_addr() as u64)
    }
}

pub fn current_cpu() -> Pin<&'static Cpu> {
    unsafe { Pin::new_unchecked(&*core::ptr::from_exposed_addr(rdmsr(KERNEL_GS_BASE) as usize)) }
}

pub fn wait_until_cpus_registered() {
    while CPU_ID_COUNTER.load(Ordering::SeqCst) != *CPU_COUNT {
        core::hint::spin_loop();
    }
}
