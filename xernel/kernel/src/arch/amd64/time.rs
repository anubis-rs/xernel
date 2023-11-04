use core::arch::asm; 

pub fn rdtsc() -> u64 {
    let ret: u64;

    unsafe {
        asm!("rdtsc", lateout("rax") ret);
    }

    ret
}
