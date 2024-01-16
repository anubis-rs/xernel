#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![feature(strict_provenance)]
#![feature(exposed_provenance)]
#![allow(dead_code)]
#![allow(clippy::fn_to_numeric_cast)]
#![allow(non_upper_case_globals)]
extern crate alloc;

#[macro_use]
mod writer;

#[macro_use]
mod logger;

mod acpi;
mod allocator;
mod arch;
mod cpu;
mod drivers;
mod framebuffer;
mod fs;
mod mem;
mod sched;
mod syscall;
mod timer;
mod utils;

use alloc::sync::Arc;
use core::arch::asm;
use core::panic::PanicInfo;
use core::time::Duration;
use libxernel::sync::Spinlock;
use limine::*;
use x86_64::instructions::interrupts;

use arch::amd64::gdt;

use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size2MiB;
use x86_64::VirtAddr;

use crate::acpi::hpet;
use crate::arch::amd64;
use crate::arch::amd64::apic;
use crate::arch::amd64::hcf;
use crate::cpu::wait_until_cpus_registered;
use crate::cpu::CPU_COUNT;
use crate::cpu::{current_cpu, register_cpu};
use crate::fs::vfs;
use crate::fs::vfs::VFS;
use crate::mem::frame::FRAME_ALLOCATOR;
use crate::mem::paging::KERNEL_PAGE_MAPPER;
use crate::sched::process::Process;
use crate::sched::process::KERNEL_PROCESS;
use crate::sched::scheduler;
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

// define the kernel's entry point function
#[no_mangle]
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

    let process = Arc::new(Spinlock::new(Process::new(Some(KERNEL_PROCESS.clone()))));

    let _user_task = Thread::new_user_thread(process.clone(), VirtAddr::new(0x200000));

    let page = FRAME_ALLOCATOR.lock().allocate_frame::<Size2MiB>().unwrap();
    KERNEL_PAGE_MAPPER.lock().map(
        page,
        Page::from_start_address(VirtAddr::new(0x200000)).unwrap(),
        PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::PRESENT,
        true,
    );
    process.lock().get_page_table().unwrap().map(
        page,
        Page::from_start_address(VirtAddr::new(0x200000)).unwrap(),
        PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::PRESENT,
        true,
    );

    // unsafe {
    //     let start_address_fn = test_userspace_fn as usize;

    //     // the `test_userspace_fn` is very small and should fit in 512 bytes
    //     for i in 0..512 {
    //         let ptr = (0x200000 + i) as *mut u8;
    //         let val = (start_address_fn + i) as *mut u8;

    //         ptr.write_volatile(val.read_volatile());
    //     }
    // }

    let main_task = Thread::kernel_thread_from_fn(kmain_thread);

    let kernel_task = Thread::kernel_thread_from_fn(task1);

    let kernel_task2 = Thread::kernel_thread_from_fn(task2);

    current_cpu().run_queue.write().push_back(Arc::new(main_task));
    current_cpu().run_queue.write().push_back(Arc::new(kernel_task));
    current_cpu().run_queue.write().push_back(Arc::new(kernel_task2));

    let timekeeper = TimerEvent::new(hardclock, (), Duration::from_secs(1), false);

    current_cpu().timer_queue.write().queue_event(timekeeper);

    let event = TimerEvent::new(reschedule, (), Duration::from_millis(5), false);

    current_cpu().timer_queue.write().queue_event(event);

    amd64::interrupts::enable();

    hcf();
}

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

#[naked]
pub extern "C" fn test_userspace_fn() {
    //loop {
    unsafe {
        asm!(
            "\
                mov rax, 0
                mov rdi, 2
                mov rsi, 3
                mov rdx, 4
                syscall
                mov rax, 0
            ",
            options(noreturn)
        );
    }
    //}
}

#[no_mangle]
fn task1() {
    let mut var = 1;

    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task1 {}", var);
        var += 1;
    }
}

fn task2() {
    let mut var = -1;

    loop {
        for _ in 0..i16::MAX {
            unsafe {
                asm!("nop");
            }
        }

        dbg!("hello from task2 {}", var);
        var -= 1;
    }
}
