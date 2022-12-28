use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, LStar, Star},
        rflags::RFlags,
    },
    VirtAddr,
};

use crate::{arch::x64::gdt::GDT_BSP, println};

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

    LStar::write(VirtAddr::new(handler as u64));

    // disable interrupts when syscall handler is called
    x86_64::registers::model_specific::SFMask::write(RFlags::INTERRUPT_FLAG);
}

fn handler() {
    // TODO: add stack for syscalls
    println!("lul hiiit");
}
