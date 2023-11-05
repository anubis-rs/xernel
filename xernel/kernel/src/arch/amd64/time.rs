use core::arch::asm;
use libxernel::sync::Once;

pub static TSC_FREQUENCY: Once<u64> = Once::new();

pub fn calibrate_tsc() {
    
}

pub fn rdtsc() -> u64 {
    let ret: u64;

    unsafe {
        asm!("rdtsc", lateout("rax") ret);
    }

    ret
}

