#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![allow(dead_code)]
#![allow(clippy::fn_to_numeric_cast)]
extern crate alloc;

#[macro_use]
mod writer;

#[macro_use]
mod logger;

mod acpi;
mod allocator;
mod arch;
mod backtrace;
mod cpu;
mod drivers;
mod framebuffer;
mod fs;
mod limine_module;
mod sched;
mod syscall;

mod mem;

use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use x86_64::VirtAddr;
use core::arch::asm;
use core::panic::PanicInfo;
use libxernel::sync::Spinlock;
use limine::*;
use x86_64::instructions::interrupts;

use arch::amd64::gdt;
use arch::amd64::idt;

use crate::acpi::hpet;
use crate::arch::amd64::apic;
use crate::cpu::register_cpu;
use crate::cpu::wait_until_cpus_registered;
use crate::cpu::CPU_COUNT;
use crate::fs::vfs;
use crate::fs::vfs::VFS;
use crate::mem::paging::KERNEL_PAGE_MAPPER;
use crate::sched::process::Process;
use crate::sched::process::KERNEL_PROCESS;
use crate::sched::scheduler;
use crate::sched::scheduler::{Scheduler, SCHEDULER};
use crate::sched::thread::Thread;

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
    idt::init();
    info!("IDT loaded");
    idt::disable_pic();

    mem::init();

    acpi::init();
    info!("acpi initialized");

    backtrace::init();
    info!("backtrace initialized");

    hpet::init();

    apic::init();

    syscall::init();

    vfs::init();

    let t = VFS.lock().vn_open("/test.txt".to_string(), 0).unwrap();

    let mut write_buf: Vec<u8> = vec![5; 10];

    VFS.lock()
        .vn_write(t.clone(), &mut write_buf)
        .expect("write to file failed");

    let mut read_buf: Vec<u8> = vec![0; 5];

    VFS.lock().vn_read(t.clone(), &mut read_buf).expect("read failed");

    println!(
        "name of fs where node is mounted: {}",
        t.lock().vfsp.upgrade().unwrap().lock().vfs_name()
    );
    println!("{:?}", write_buf);
    println!("{:?}", read_buf);

    let bootloader_info = BOOTLOADER_INFO
        .get_response()
        .get()
        .expect("barebones: recieved no bootloader info");

    info!(
        "bootloader: (name={:?}, version={:?})",
        bootloader_info.name.to_str().unwrap(),
        bootloader_info.version.to_str().unwrap()
    );

    let smp_response = SMP_REQUEST.get_response().get_mut().unwrap();

    let bsp_lapic_id = smp_response.bsp_lapic_id;

    CPU_COUNT.set_once(smp_response.cpu_count as usize);

    register_cpu();

    for cpu in smp_response.cpus().iter_mut() {
        if cpu.lapic_id != bsp_lapic_id {
            cpu.goto_address = arch::amd64::x86_64_ap_main;
        }
    }

    KERNEL_PROCESS.set_once(Arc::new(Spinlock::new(Process::new(None))));

    wait_until_cpus_registered();

    scheduler::init();

    let process = Arc::new(Spinlock::new(Process::new(Some(KERNEL_PROCESS.clone()))));
    dbg!("before pt load");
    unsafe {
        process.lock().page_table.as_ref().unwrap().load_pt();
        dbg!("ffffffff80016140 => {:x}", KERNEL_PAGE_MAPPER.lock().translate(VirtAddr::new(0xffffffff80016140)).unwrap().as_u64());
        dbg!("ffffffff80016140 => {:x}", process.lock().page_table.as_ref().unwrap().translate(VirtAddr::new(0xffffffff80016140)).unwrap().as_u64());
    }
    dbg!("after pt load");

    let test_elf = include_bytes!("../test-elfloader");
    let user_task = Thread::new_user_thread_from_elf(process.clone(), test_elf);
    let main_task = Thread::kernel_thread_from_fn(kernel_main_task);
    let kernel_task = Thread::kernel_thread_from_fn(task1);
    let kernel_task2 = Thread::kernel_thread_from_fn(task2);

    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(main_task)));
    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(user_task)));
    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(kernel_task)));
    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(kernel_task2)));

    unsafe {
        for (i, sched) in SCHEDULER.get_all().iter().enumerate() {
            println!("cpu {} has {} tasks", i, sched.lock().threads.len());
        }
    }

    Scheduler::hand_over();

    unreachable!();
}

pub fn kernel_main_task() {
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
