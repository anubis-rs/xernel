#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(pointer_byte_offsets)]

extern crate alloc;

#[macro_use]
extern crate lazy_static;

mod acpi;
mod arch;
mod framebuffer;

#[macro_use]
mod logger;
mod mem;

#[macro_use]
mod writer;

use core::arch::asm;
use core::panic::PanicInfo;
use limine::*;

use arch::x64::gdt;
use arch::x64::idt;

use mem::{heap, pmm, vmm};
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::FrameDeallocator;

use crate::acpi::hpet;
use crate::arch::x64::apic;
use crate::arch::x64::apic::APIC;

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // print the panic info
    // NOTE: this might panic again, but it is better than printing nothing
    println!("\nKernel PANIC !!!");
    println!("panic info: {:#?}", info);
    loop {}
}

// define the kernel's entry point function
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    gdt::init();
    idt::init();
    println!("GDT loaded");

    idt::disable_pic();

    pmm::init();
    println!("pm initialized");

    // test allocate a page
    let mut frame_allocator = pmm::FRAME_ALLOCATOR.lock();

    unsafe {
        let frame = frame_allocator.allocate_frame().unwrap();
        frame_allocator.deallocate_frame(frame);
    }

    drop(frame_allocator);

    vmm::init();
    println!("vm initialized");

    heap::init();
    println!("heap initialized");

    acpi::init();
    println!("acpi initialized");

    hpet::init();

    apic::init();

    use alloc::boxed::Box;

    let mut test_allocation = Box::new(42);
    println!("test allocation: {}", test_allocation);
    *test_allocation = 123;
    println!("test allocation: {}", test_allocation);

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("barebones: recieved no bootloader info");

    println!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_string().unwrap(),
        bootloader_info.version.to_string().unwrap()
    );

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
