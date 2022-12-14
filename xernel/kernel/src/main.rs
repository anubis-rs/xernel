#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(pointer_byte_offsets)]
#![feature(naked_functions)]
#![feature(exclusive_range_pattern)]
#![allow(dead_code)]
#![allow(clippy::fn_to_numeric_cast)]
extern crate alloc;

#[macro_use]
extern crate lazy_static;

mod acpi;
mod allocator;
mod arch;
mod drivers;
mod framebuffer;
mod sched;

#[macro_use]
mod logger;
mod mem;

#[macro_use]
mod writer;

use core::arch::asm;
use core::panic::PanicInfo;
use limine::*;
use x86_64::instructions::interrupts;

use arch::x64::gdt;
use arch::x64::idt;

use mem::{heap, pmm, vmm};
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
    error!("Kernel PANIC !!!");
    error!("panic info: {:#?}", info);
    loop {}
}

// define the kernel's entry point function
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    //framebuffer::show_start_image();

    gdt::init();
    info!("GDT loaded");
    idt::init();
    info!("IDT loaded");
    idt::disable_pic();

    pmm::init();
    info!("pm initialized");

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

    let main_task = Task::new_kernel_task(VirtAddr::new(0));

    let kernel_task = Task::kernel_task_from_fn(task1);

    let kernel_task2 = Task::kernel_task_from_fn(task2);

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

    let mut var = 1;

    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from main {}", var);
        var += 1;
    }
}

#[no_mangle]
fn task1() {
    let mut var = 1;

    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task1 {}", var);
        var += 1;
    }
}

fn task2() {
    let mut var = -1;

    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task2 {}", var);
        var -= 1;
    }
}
