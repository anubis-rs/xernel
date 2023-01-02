use core::arch::asm;
use libxernel::syscall::{SyscallError, SYS_READ, SYS_WRITE};
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
    syscall_number: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    eflags: usize,
    return_address: usize,
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

#[naked]
unsafe extern "C" fn asm_syscall_handler() {
    asm!(
        "
    swapgs # gs contains the stackpointer for this thread now

    mov gs:0, rsp # save the stackpointer for this task
    mov rsp, gs:16 # load the kernel stackpointer for this task

    # backup registers for sysretq
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

    mov rsp, gs:0 # load the stackpointer for this task

    swapgs
    sysretq
    ",
        options(noreturn)
    );
}

fn sys_read(fd: usize, buf: &mut [u8]) -> Result<isize, SyscallError> {
    Ok((fd * buf.len()) as isize)
}

fn syscall_arg_to_slice<'a, T>(ptr: usize, len: usize) -> &'a mut [T] {
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut T, len) }
}

fn syscall_arg_to_reference<'a, T>(ptr: usize) -> &'a mut T {
    unsafe { &mut *(ptr as *mut T) }
}

#[no_mangle]
extern "sysv64" fn general_syscall_handler(data: SyscallData) -> i64 {
    println!("general_syscall_handler: {:#x?}", data);

    let result = match data.syscall_number as usize {
        SYS_READ => sys_read(data.arg0, syscall_arg_to_slice(data.arg1, data.arg2)),
        SYS_WRITE => todo!("write"),
        _ => {
            unimplemented!("unknown syscall: {:x?}", data);
        }
    };

    match result {
        Ok(value) => value as i64,
        Err(error) => error as i64,
    }
}
