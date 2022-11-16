use x86_64::VirtAddr;

#[repr(C)]
pub struct TaskContext {
    pub ds: u64,
    pub es: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub err: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl TaskContext {
    pub fn new(entry_point: VirtAddr, rsp: VirtAddr, rflags: u64) -> Self {
        Self {
            ds: 0,
            es: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            err: 0,
            rip: entry_point.as_u64(),
            cs: 0,
            rflags: rflags,
            rsp: rsp.as_u64(),
            ss: 0,
        }
    }
}
