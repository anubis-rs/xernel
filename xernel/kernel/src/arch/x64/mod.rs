pub mod apic;
pub mod gdt;
pub mod idt;
pub mod ports;

use crate::arch::x64::apic::APIC;
use crate::cpu::register_cpu;
use crate::sched::scheduler::{Scheduler, SCHEDULER};
use crate::KERNEL_PAGE_MAPPER;
use limine::SmpInfo;

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
