use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};
use libxernel::sync::Once;
use x86_64::registers::model_specific::KernelGsBase;
use x86_64::VirtAddr;

use crate::arch::x64::apic::APIC;

static CPU_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub static CPU_COUNT: Once<usize> = Once::new();

#[repr(packed)]
pub struct CpuData {
    // NOTE: don't move this variables as we need to access them from assembly
    user_space_stack: usize,
    kernel_stack: usize,

    cpu_id: usize,
    lapic_id: u32,
}

impl CpuData {
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

    let cpu_data = Box::leak(Box::new(CpuData {
        user_space_stack: 0,
        kernel_stack: 0,
        cpu_id,
        lapic_id,
    }));

    // use KERNEL_GS_BASE to store the cpu_data
    KernelGsBase::write(VirtAddr::new(cpu_data as *const _ as u64));
}

pub fn get_cpu_data() -> &'static mut CpuData {
    let cpu_data = KernelGsBase::read().as_u64() as *mut CpuData;
    unsafe { &mut *cpu_data }
}
