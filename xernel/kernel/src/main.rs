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

    let main_task = Task::new_kernel_task(VirtAddr::new(0), VirtAddr::new(0), 0);

    let stack1 = vec![0; 4096];

    let kernel_task = Task::new_kernel_task(
        VirtAddr::new(task1 as *const () as u64),
        VirtAddr::new(stack1.as_ptr() as u64 + 4096), // stack grows down
        0,
    );

    let stack2 = vec![0; 4096];

    let kernel_task2 = Task::new_kernel_task(
        VirtAddr::new(task2 as *const () as u64),
        VirtAddr::new(stack2.as_ptr() as u64 + 4096), // stack grows down
        0,
    );

    SCHEDULER.lock().add_task(main_task);
    SCHEDULER.lock().add_task(kernel_task);
    SCHEDULER.lock().add_task(kernel_task2);

    interrupts::enable();

    let smp_response = SMP_REQUEST.get_response().get_mut().unwrap();

    let bsp_lapic_id = smp_response.bsp_lapic_id;

    for cpu in smp_response.cpus().iter_mut() {
        if cpu.lapic_id != bsp_lapic_id {
            cpu.goto_address = arch::x64::x86_64_ap_main;
        }
    }

    let mut var = 5;

    loop {
        for i in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from main {}", var);
        var += 1;
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

fn task1() {
    let mut var = 5;

    loop {
        for i in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task1 {}", var);
        var += 1;
    }
}

fn task2() {
    let mut var = -5;

    loop {
        for i in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task2 {}", var);
        var -= 1;
    }
}
