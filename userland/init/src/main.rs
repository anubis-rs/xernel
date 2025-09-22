#![no_std]
#![no_main]

use core::arch::asm;

#[panic_handler]
fn panic(__info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        main();
    }
}

fn main() {
    let hello_str = "Hello from userspace :)\0";
    unsafe {
        asm!(
            "syscall",
            in("rdi") hello_str.as_ptr(),
            in("rax") 5
        );
    }
}
