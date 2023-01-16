#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(pointer_byte_offsets)]
#![feature(naked_functions)]
#![feature(exclusive_range_pattern)]
#![feature(let_chains)]
#![allow(dead_code)]
#![allow(clippy::fn_to_numeric_cast)]
extern crate alloc;

#[macro_use]
extern crate lazy_static;

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

#[macro_use]
mod logger;
mod mem;

#[macro_use]
mod writer;

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use core::panic::PanicInfo;
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
use crate::sched::scheduler::{Scheduler, SCHEDULER};
use crate::sched::task::Task;
use crate::sched::task::TaskStatus;

static BOOTLOADER_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);
static SMP_REQUEST: LimineSmpRequest = LimineSmpRequest::new(0);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
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
    framebuffer::show_start_image();

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

    let user_task = Task::new_user_task(VirtAddr::new(0x200000));

    let page = FRAME_ALLOCATOR.lock().allocate_frame::<Size2MiB>().unwrap();
    KERNEL_PAGE_MAPPER.lock().map(
        page,
        Page::from_start_address(VirtAddr::new(0x200000)).unwrap(),
        PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::PRESENT,
        true,
    );
    user_task.get_page_table().unwrap().map(
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

    let mut main_task = Task::new_kernel_task(VirtAddr::new(0));
    main_task.status = TaskStatus::Running;

    let kernel_task = Task::kernel_task_from_fn(task1);

    let kernel_task2 = Task::kernel_task_from_fn(task2);

    SCHEDULER.get().lock().add_task(main_task);
    //SCHEDULER.get().lock().add_task(user_task);
    SCHEDULER.get().lock().add_task(kernel_task);
    SCHEDULER.get().lock().add_task(kernel_task2);

    interrupts::enable();

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
