use core::arch::asm;
use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, LStar, Star},
        rflags::RFlags,
    },
    VirtAddr,
};

use crate::arch::x64::gdt::GDT_BSP;

pub fn init() {
    // set IA32_STAR
    Star::write(
        GDT_BSP.1.user_code_selector,
        GDT_BSP.1.user_data_selector,
        GDT_BSP.1.code_selector,
        GDT_BSP.1.data_selector,
    )
    .unwrap();

    // enable IA32_EFER
    unsafe {
        Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
    }

    LStar::write(VirtAddr::new(syscall_handler as u64));

    // disable interrupts when syscall handler is called
    x86_64::registers::model_specific::SFMask::write(RFlags::INTERRUPT_FLAG);
}

#[naked]
unsafe extern "C" fn syscall_handler() {
    asm!(
        "
    swapgs # gs contains the stackpointer for this thread now

    mov gs:0, rsp # save the stackpointer for this task
    mov rsp, gs:16 # load the kernel stackpointer for this task

    nop
    nop

    mov rsp, gs:0 # load the stackpointer for this task

    swapgs
    sysret
    ",
        options(noreturn)
    );
}
