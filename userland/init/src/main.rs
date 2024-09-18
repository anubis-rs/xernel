#![no_std]
#![no_main]

use core::arch::asm;

#[panic_handler]
fn panic(__info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    main();
    loop { }
}

fn main() {
    let hello_str = "Hello from Init\0";
    unsafe {
        asm!(
            "\
                mov rax, 5
                mov rdi, {0}
                syscall
            ",
            in(reg) hello_str.as_ptr(), options(noreturn)
        );
    }
}
