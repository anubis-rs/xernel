#![no_std]
#![no_main]

mod writer;
mod framebuffer;

use core::panic::PanicInfo;
use limine::*;
use core::arch::asm;

use crate::framebuffer::{printc, init_framebuffer};

static TERMINAL_REQUEST: LimineTerminalRequest = LimineTerminalRequest::new(0);
static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static MMAP: LimineMmapRequest = LimineMmapRequest::new(0);

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
    println!("Hello, rusty world!");

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("barebones: recieved no bootloader info");

    println!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_string().unwrap(),
        bootloader_info.version.to_string().unwrap()
    );

    let mmap = MMAP
        .get_response()
        .get()
        .expect("barebones: recieved no mmap")
        .mmap();

    println!("mmap: {:#x?}", mmap);

    unsafe {
        init_framebuffer();
        printc();
    }

    loop {
        unsafe { asm!("hlt"); }
    }
}
