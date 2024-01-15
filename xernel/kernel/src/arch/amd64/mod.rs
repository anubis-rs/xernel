pub mod apic;
pub mod gdt;
pub mod idt;
pub mod ports;
pub mod time;
mod lapic;
mod ioapic;
pub mod cpuid;

use crate::arch::amd64::apic::APIC;
use crate::cpu::register_cpu;
use crate::sched::scheduler::{Scheduler, SCHEDULER};
use crate::{KERNEL_PAGE_MAPPER, info};
use core::arch::asm;
use limine::SmpInfo;
use x86_64::VirtAddr;
use crate::sched::context::Context;

pub enum IPL {
    IPL0,
    IPLAPC,
    IPLDPC,
    IPLDevice,
    IPLClock,
    IPLHigh
}

#[no_mangle]
pub extern "C" fn x86_64_ap_main(boot_info: *const SmpInfo) -> ! {
    let boot_info = unsafe { &*boot_info };
    let ap_id = boot_info.processor_id as usize;

    {
        let kernel_page_mapper = KERNEL_PAGE_MAPPER.lock();
        unsafe {
            kernel_page_mapper.load_pt();
        }
    }

    info!("booting CPU {:?}", boot_info);

    gdt::init_ap(ap_id);
    info!("CPU{}: gdt initialized", ap_id);

    idt::init();
    info!("CPU{}: idt initialized", ap_id);

    register_cpu();
    info!("CPU{}: cpu registered", ap_id);

    // wait until all CPUs are registered before scheduling
    SCHEDULER.wait_until_initialized();

    APIC.enable_apic();
    info!("CPU{}: apic initialized", ap_id);

    Scheduler::hand_over();

    unreachable!()
}

#[inline]
pub fn read_cr2() -> VirtAddr {
    let value: u64;

    unsafe {
        asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));

        VirtAddr::new(value)
    }
}

#[inline]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
}

#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    unsafe {
        asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr);
    }
    ((high as u64) << 32) | (low as u64)
}

pub fn switch_context(old_ctx: *mut *mut Context, new_ctx: *mut Context) {
    // TODO: Use inline assembly to perform context switch
}
