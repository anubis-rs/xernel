#![no_std]
#![no_main]

mod writer;

use core::panic::PanicInfo;
use limine::*;

static TERMINAL_REQUEST: LimineTerminalRequest = LimineTerminalRequest::new(0);
static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static MMAP: LimineMmapRequest = LimineMmapRequest::new(0);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// define the kernel's entry point function
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    println!("Hello, rusty world!\n");

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

    loop {}
}