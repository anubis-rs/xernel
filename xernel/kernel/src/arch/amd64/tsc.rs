use core::{
    arch::asm,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::acpi::hpet;

pub static TSC_TICKS_PER_MS: AtomicU64 = AtomicU64::new(0);

// TODO: Use TSC Deadshot mode for apic
pub fn calibrate_tsc() {
    let start: u64 = rdtsc();
    hpet::sleep(10_000_000);
    let end: u64 = rdtsc();

    println!("start: {} end: {}", start, end);

    let ticks_per_ms = (end - start) / 10;

    println!("ticks_per_ms: {}", ticks_per_ms);

    TSC_TICKS_PER_MS.store(ticks_per_ms, Ordering::SeqCst);
}

pub fn rdtsc() -> u64 {
    let ret: u64;

    unsafe {
        asm!("rdtsc", lateout("rax") ret);
    }

    ret
}
