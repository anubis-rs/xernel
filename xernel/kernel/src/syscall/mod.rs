use core::arch::asm;
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

    LStar::write(VirtAddr::new(asm_syscall_handler as u64));

    // disable interrupts when syscall handler is called
    x86_64::registers::model_specific::SFMask::write(RFlags::INTERRUPT_FLAG);
}

#[derive(Debug)]
#[repr(C)]
struct SyscallData {
    syscall_number: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    eflags: u64,
    return_address: u64,
}

/*
 * Register setup:
 * rax  system call number
 * rdi  arg0
 * rsi  arg1
 * rdx  arg2
 * r10  arg3
 * r8   arg4
 * r9   arg5
 * r11  eflags for syscall/sysret
 * rcx  return address for syscall/sysret
 */

// FIXME: currently some registers are saved twice
#[naked]
unsafe extern "C" fn asm_syscall_handler() {
    asm!(
        "
    swapgs # gs contains the stackpointer for this thread now

    mov gs:0, rsp # save the stackpointer for this task
    mov rsp, gs:16 # load the kernel stackpointer for this task

    push rcx # backup registers for sysretq
    push r11
    push rbp
    push rbx # save callee-saved registers
    push r12
    push r13
    push r14
    push r15

    # save the syscall data
    push rcx
    push r11
    push r9
    push r8
    push r10
    push rdx
    push rsi
    push rdi
    push rax

    mov rdi, rsp # pass the SyscallData struct to the syscall handler

    sti # enable interrupts

    call general_syscall_handler

    cli # disable interrupts, interrupts are automatically re-enabled when the syscall handler returns

    # restore the syscall data
    pop rdi # we don't restore rax as it's the return value of the syscall
    pop rdi
    pop rsi
    pop rdx
    pop r10
    pop r8
    pop r9
    pop r11
    pop rcx

    pop r15 # restore callee-saved registers
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp # restore stack and registers for sysretq
    pop r11
    pop rcx

    mov rsp, gs:0 # load the stackpointer for this task

    swapgs
    sysretq
    ",
        options(noreturn)
    );
}

#[no_mangle]
extern "sysv64" fn general_syscall_handler(data: SyscallData) -> u64 {
    println!("general_syscall_handler: {:#x?}", data);

    1
}
