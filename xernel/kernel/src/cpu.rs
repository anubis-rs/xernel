use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use libxernel::sync::Once;
use x86_64::registers::model_specific::KernelGsBase;
use x86_64::VirtAddr;

use crate::arch::x64::apic::APIC;

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

    pub fn wait_until_cpus_registered(&self) {
        while CPU_ID_COUNTER.load(Ordering::SeqCst) != *CPU_COUNT {
            core::hint::spin_loop();
        }
    }

    pub fn init(&self, init_fn: fn() -> T) {
        assert_eq!(*CPU_COUNT, CPU_ID_COUNTER.load(Ordering::SeqCst));

        let vec = unsafe { &mut *self.data.get() };

        for _ in 0..*CPU_COUNT {
            vec.push(init_fn());
        }
    }

    fn check_initialized(&self) {
        let vec = unsafe { &*self.data.get() };

        assert_eq!(vec.len(), *CPU_COUNT);
    }

    pub fn get(&self) -> &T {
        self.check_initialized();

        let cpu_id = get_per_cpu_data().get_cpu_id();
        let vec = unsafe { &mut *self.data.get() };
        &vec[cpu_id]
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        self.check_initialized();

        let cpu_id = get_per_cpu_data().get_cpu_id();
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

#[repr(packed)]
pub struct PerCpuData {
    // NOTE: don't move these variables as we need to access them from assembly
    user_space_stack: usize,
    kernel_stack: usize,

    cpu_id: usize,
    lapic_id: u32,
}

impl PerCpuData {
    pub fn set_kernel_stack(&mut self, kernel_stack: usize) {
        self.kernel_stack = kernel_stack;
    }

    pub fn get_cpu_id(&self) -> usize {
        self.cpu_id
    }
}

pub fn register_cpu() {
    let cpu_id = CPU_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let lapic_id = APIC.lapic_id();

    let cpu_data = Box::leak(Box::new(PerCpuData {
        user_space_stack: 0,
        kernel_stack: 0,
        cpu_id,
        lapic_id,
    }));

    // use KERNEL_GS_BASE to store the cpu_data
    KernelGsBase::write(VirtAddr::new(cpu_data as *const _ as u64));
}

pub fn get_per_cpu_data() -> &'static mut PerCpuData {
    let cpu_data = KernelGsBase::read().as_u64() as *mut PerCpuData;
    unsafe { &mut *cpu_data }
}
