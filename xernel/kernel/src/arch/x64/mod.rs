pub mod apic;
pub mod gdt;
pub mod idt;
pub mod ports;

use crate::info;
use crate::KERNEL_PAGE_MAPPER;
use core::arch::asm;
use limine::LimineSmpInfo;

#[no_mangle]
pub extern "C" fn x86_64_ap_main(boot_info: *const LimineSmpInfo) -> ! {
    let boot_info = unsafe { &*boot_info };
    let ap_id = boot_info.processor_id as usize;

    {
        let kernel_page_mapper = KERNEL_PAGE_MAPPER.lock();
        unsafe {
            kernel_page_mapper.load_pt();
        }
    }

    info!("booting CPU {:#?}", boot_info);

    gdt::init_ap(ap_id);
    info!("CPU{}: gdt initialized", ap_id);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
