#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(pointer_byte_offsets)]
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
use core::arch::asm;
use core::panic::PanicInfo;
use libxernel::sync::Spinlock;
use libxernel::sync::SpinlockIRQ;
use limine::*;
use x86_64::instructions::interrupts;

use arch::x64::gdt;
use arch::x64::idt;

use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size2MiB;
use x86_64::VirtAddr;

use crate::acpi::hpet;
use crate::arch::x64::apic;
use crate::cpu::register_cpu;
use crate::cpu::CPU_COUNT;
use crate::fs::vfs;
use crate::fs::vfs::VFS;
use crate::mem::pmm::FRAME_ALLOCATOR;
use crate::mem::vmm::KERNEL_PAGE_MAPPER;
use crate::sched::process::Process;
use crate::sched::scheduler::{Scheduler, SCHEDULER};
use crate::sched::thread::Thread;

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static SMP_REQUEST: LimineSmpRequest = LimineSmpRequest::new(0);

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

    VFS.lock()
        .vn_read(t.clone(), &mut read_buf)
        .expect("read failed");

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
            cpu.goto_address = arch::x64::x86_64_ap_main;
        }
    }

    SCHEDULER.wait_until_cpus_registered();
    SCHEDULER.init(|| SpinlockIRQ::new(Scheduler::new()));

    let process = Arc::new(Spinlock::new(Process::new()));

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

    unsafe {
        let start_address_fn = test_userspace_fn as usize;

        // the `test_userspace_fn` is very small and should fit in 512 bytes
        for i in 0..512 {
            let ptr = (0x200000 + i) as *mut u8;
            let val = (start_address_fn + i) as *mut u8;

            ptr.write_volatile(val.read_volatile());
        }
    }

    let main_task = Thread::kernel_thread_from_fn(process.clone(), kernel_main_task);

    let kernel_task = Thread::kernel_thread_from_fn(process.clone(), task1);

    let kernel_task2 = Thread::kernel_thread_from_fn(process, task2);

    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(main_task)));
    //Scheduler::add_task_balanced(Arc::new(Spinlock::new(user_task)));
    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(kernel_task)));
    Scheduler::add_thread_balanced(Arc::new(Spinlock::new(kernel_task2)));

    unsafe {
        for (i, sched) in SCHEDULER.get_all().iter().enumerate() {
            println!("cpu {} has {} tasks", i, sched.lock().tasks.len());
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
