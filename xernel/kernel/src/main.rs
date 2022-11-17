#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(pointer_byte_offsets)]
#![feature(naked_functions)]
#![allow(dead_code)]

extern crate alloc;

#[macro_use]
extern crate lazy_static;

mod acpi;
mod arch;
mod framebuffer;
mod sched;

#[macro_use]
mod logger;
mod mem;

#[macro_use]
mod writer;

use alloc::vec;
use core::arch::asm;
use core::panic::PanicInfo;
use limine::*;

use arch::x64::gdt;
use arch::x64::idt;

use mem::{heap, pmm, vmm};
use x86_64::instructions::interrupts;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::FrameDeallocator;
use x86_64::VirtAddr;

use crate::acpi::hpet;
use crate::arch::x64::apic;
use crate::mem::vmm::KERNEL_PAGE_MAPPER;
use crate::sched::scheduler::SCHEDULER;
use crate::sched::task::Task;

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static SMP_REQUEST: LimineSmpRequest = LimineSmpRequest::new(0);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // print the panic info
    // NOTE: this might panic again, but it is better than printing nothing
    error!("\nKernel PANIC !!!");
    error!("panic info: {:#?}", info);
    loop {}
}

// define the kernel's entry point function
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    gdt::init();
    info!("GDT loaded");
    idt::init();
    info!("IDT loaded");

    idt::disable_pic();

    pmm::init();
    info!("pm initialized");

    // test allocate a page
    let mut frame_allocator = pmm::FRAME_ALLOCATOR.lock();

    unsafe {
        let frame = frame_allocator.allocate_frame().unwrap();
        frame_allocator.deallocate_frame(frame);
    }

    drop(frame_allocator);

    vmm::init();
    info!("vm initialized");

    heap::init();
    info!("heap initialized");

    acpi::init();
    info!("acpi initialized");

    hpet::init();

    apic::init();

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("barebones: recieved no bootloader info");

    info!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_str().unwrap(),
        bootloader_info.version.to_str().unwrap()
    );

    let stack = vec![0; 4096];

    let kernel_task = Task::new_kernel_task(
        VirtAddr::new(test as *const () as u64),
        VirtAddr::new(stack.as_ptr() as u64 + 4096), // stack grows down
        0,
    );

    SCHEDULER.lock().add_task(kernel_task);

    interrupts::enable();

    let smp_response = SMP_REQUEST.get_response().get_mut().unwrap();

    let bsp_lapic_id = smp_response.bsp_lapic_id;

    for cpu in smp_response.cpus().iter_mut() {
        if cpu.lapic_id != bsp_lapic_id {
            cpu.goto_address = x86_64_ap_main;
        }
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
extern "C" fn x86_64_ap_main(boot_info: *const LimineSmpInfo) -> ! {
    let boot_info = unsafe { &*boot_info };
    let ap_id = boot_info.processor_id as usize;

    {
        let mut kernel_page_mapper = KERNEL_PAGE_MAPPER.lock();
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

fn test() {
    println!("hello");

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
