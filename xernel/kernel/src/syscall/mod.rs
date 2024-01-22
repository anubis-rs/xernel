use alloc::{
    ffi::CString,
    string::{String, ToString},
};
use core::{arch::asm, ffi::c_char};
use libxernel::syscall::{SyscallError, SYS_CLOSE, SYS_MMAP, SYS_OPEN, SYS_READ, SYS_WRITE};
use x86_64::{
    registers::{
        model_specific::{Efer, EferFlags, LStar, Star},
        rflags::RFlags,
    },
    VirtAddr,
};

use crate::{
    arch::amd64::gdt::GDT_BSP,
    fs::{self, vfs_syscalls},
    mem::mmap::mmap,
};

impl From<fs::Error> for SyscallError {
    fn from(err: fs::Error) -> SyscallError {
        match err {
            fs::Error::VNodeNotFound => SyscallError::VNodeNotFound,
            fs::Error::NotADirectory => SyscallError::NotADirectory,
            fs::Error::IsADirectory => SyscallError::IsADirectory,
            fs::Error::NoSpace => SyscallError::NoSpace,
            fs::Error::NotEmpty => SyscallError::NotEmpty,
            fs::Error::EntryNotFound => SyscallError::EntryNotFound,
            fs::Error::MountPointNotFound => SyscallError::MountPointNotFound,
            fs::Error::FileSystemNotFound => SyscallError::FileSystemNotFound,
        }
    }
}

pub type Result<T, E = SyscallError> = core::result::Result<T, E>;

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
    mov rsp, gs:8 # load the kernel stackpointer for this task

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

fn syscall_arg_to_slice<'a, T>(ptr: usize, len: usize) -> &'a mut [T] {
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut T, len) }
}

fn syscall_arg_to_reference<'a, T>(ptr: usize) -> &'a mut T {
    unsafe { &mut *(ptr as *mut T) }
}

fn syscall_arg_to_string(ptr: usize) -> Option<String> {
    unsafe { Some(CString::from_raw(ptr as *mut c_char).to_str().ok()?.to_string()) }
}

#[no_mangle]
extern "sysv64" fn general_syscall_handler(data: SyscallData) -> i64 {
    // println!("general_syscall_handler: {:#x?}", data);

    let result = match data.syscall_number {
        SYS_READ => vfs_syscalls::sys_read(data.arg0, syscall_arg_to_slice(data.arg1, data.arg2)),
        SYS_WRITE => vfs_syscalls::sys_write(data.arg0, syscall_arg_to_slice(data.arg1, data.arg2)),
        SYS_OPEN => {
            let path = syscall_arg_to_string(data.arg0);

            match path {
                Some(path) => vfs_syscalls::sys_open(path, data.arg1 as u64),
                None => Err(SyscallError::MalformedPath),
            }
        }
        SYS_CLOSE => vfs_syscalls::sys_close(data.arg0),
        SYS_MMAP => mmap(data.arg0, data.arg1, data.arg2, data.arg3, data.arg4, data.arg5),
        _ => {
            unimplemented!("unknown syscall: {:x?}", data);
        }
    };

    match result {
        Ok(value) => value as i64,
        Err(error) => error as i64,
    }
}
