#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)]
#![allow(clippy::fn_to_numeric_cast)]
#![allow(non_upper_case_globals)]
extern crate alloc;

#[macro_use]
mod writer;

#[macro_use]
mod logger;

#[macro_use]
mod utils;

mod acpi;
mod allocator;
mod arch;
mod cpu;
mod dpc;
mod drivers;
mod framebuffer;
mod fs;
mod mem;
mod sched;
mod syscall;
mod timer;
mod userland;

use alloc::sync::Arc;
use core::arch::asm;
use core::panic::PanicInfo;
use core::time::Duration;
use fs::initramfs;
use libxernel::sync::Spinlock;
use limine::*;
use x86_64::instructions::interrupts;

use arch::amd64::gdt;

use crate::acpi::hpet;
use crate::arch::amd64;
use crate::arch::amd64::apic;
use crate::arch::amd64::hcf;
use crate::cpu::CPU_COUNT;
use crate::cpu::wait_until_cpus_registered;
use crate::cpu::{current_cpu, register_cpu};
use crate::fs::vfs;
use crate::fs::vfs::VFS;
use crate::mem::paging::KERNEL_PAGE_MAPPER;
use crate::sched::process::KERNEL_PROCESS;
use crate::sched::process::Process;
use crate::sched::scheduler::reschedule;
use crate::sched::thread::Thread;
use crate::timer::hardclock;
use crate::timer::timer_event::TimerEvent;
use crate::utils::backtrace;
use crate::utils::rtc::Rtc;
static BOOTLOADER_INFO: BootInfoRequest = BootInfoRequest::new(0);
static SMP_REQUEST: SmpRequest = SmpRequest::new(0);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // disable interrupts in panic handler to prevent getting scheduled again
    interrupts::disable();

    // TODO: check which task paniced and kill it

    dbg!("Kernel PANIC !!!");
    dbg!("panic info: {:#?}", info);

    // print the panic info
    // NOTE: this might panic again, but it is better than printing nothing
    error!("Kernel PANIC !!!");
    error!("panic info: {:#?}", info);
    loop {}
}

// TODO: Use the discussed solution for TSS
// TODO: Proper Error handling across the whole kernel (error enums etc.)
// TODO: Replace linked_list_allocator with a self written allocator
// TODO: Implement VFS correctly and devFS
// TODO: Implement tmpfs
// TODO: Convenience functions for creating timer and directly adding it to the queue
// TODO: Same for dpcs
// FIXME: Fix deadlock which occures after some time in timer interrupt handler
// TODO: Implement some sort of overview which threads holds which locks

// define the kernel's entry point function
#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    framebuffer::init();
    info!("framebuffer initialized");

    gdt::init();
    info!("GDT loaded");
    amd64::interrupts::init();
    info!("IDT loaded");
    amd64::interrupts::disable_pic();

    mem::init();

    acpi::init();
    info!("acpi initialized");

    backtrace::init();
    info!("backtrace initialized");

    hpet::init();

    apic::init();

    syscall::init();

    vfs::init();

    vfs::test();

    initramfs::load_initramfs();
    info!("initramfs loaded");

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("barebones: recieved no bootloader info");

    info!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_str().unwrap(),
        bootloader_info.version.to_str().unwrap()
    );

    Rtc::read();

    KERNEL_PROCESS.set_once(Arc::new(Spinlock::new(Process::new(None))));

    let smp_response = SMP_REQUEST.get_response().get_mut().unwrap();

    let bsp_lapic_id = smp_response.bsp_lapic_id;

    CPU_COUNT.set_once(smp_response.cpu_count as usize);

    register_cpu();

    for cpu in smp_response.cpus().iter_mut() {
        if cpu.lapic_id != bsp_lapic_id {
            cpu.goto_address = arch::amd64::x86_64_ap_main;
        }
    }

    wait_until_cpus_registered();

    timer::init();
    info!("scheduler initialized");

    let main_task = Thread::kernel_thread_from_fn(kmain_thread);

    current_cpu().enqueue_thread(Arc::new(main_task));

    userland::init();
    info!("userland initialized");

    let timekeeper = TimerEvent::new(hardclock, (), Duration::from_secs(1), false);

    current_cpu().enqueue_timer(timekeeper);

    let resched = TimerEvent::new(reschedule, (), Duration::from_millis(5), false);

    current_cpu().enqueue_timer(resched);

    amd64::interrupts::enable();
    hcf();
}

// TODO: Do something useful in the kernel main thread
pub fn kmain_thread() {
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
