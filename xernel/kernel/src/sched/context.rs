use core::arch::asm;
use crate::arch::ExceptionContext;

#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
/// Represents a Thread Context which gets saved on a context switch
pub struct ThreadContext {
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl ThreadContext {
    /// Creates a new, zero-initialized context
    pub const fn new() -> Self {
        Self {
            rbp: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: 0,
            cs: 0,
            rflags: 0,
            rsp: 0,
            ss: 0,
        }
    }
}

impl From<ExceptionContext> for ThreadContext {
    fn from(value: ExceptionContext) -> Self {
        Self {
            rbp: value.rbp,
            rax: value.rax,
            rbx: value.rbx,
            rcx: value.rcx,
            rdx: value.rdx,
            rsi: value.rsi,
            rdi: value.rdi,
            r8: value.r8,
            r9: value.r9,
            r10: value.r10,
            r11: value.r11,
            r12: value.r12,
            r13: value.r13,
            r14: value.r14,
            r15: value.r15,
            rip: value.rip,
            cs: value.cs,
            rflags: value.rflags,
            rsp: value.rsp,
            ss: value.ss,
        }
    }
}

#[naked]
/// Restores the gives context and jumps to new RIP via iretq
pub extern "C" fn restore_context(ctx: *const ThreadContext) -> ! {
    unsafe {
        asm!(
            "mov rsp, rdi;
            pop rbp;
            pop rax;
            pop rbx;
            pop rcx;
            pop rdx;
            pop rsi;
            pop rdi;
            pop r8;
            pop r9;
            pop r10;
            pop r11;
            pop r12;
            pop r13;
            pop r14;
            pop r15;
            iretq;",
            options(noreturn)
        );
    }
}
