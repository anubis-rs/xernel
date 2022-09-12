#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate lazy_static;

mod arch;
mod framebuffer;

#[macro_use]
mod logger;
mod mem;
mod writer;

use core::arch::asm;
use core::panic::PanicInfo;
use limine::*;

use arch::x64::gdt;
use arch::x64::idt;

use mem::pmm;

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
    println!("Hello");

    dbg!("hello");
    dbg!("hello {}", "world");

    gdt::init();
    idt::init();
    println!("GDT loaded");

    pmm::init();
    println!("pm initialized");

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
