use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::sync::Weak;
use core::cell::{Cell, UnsafeCell};
use core::pin::Pin;

use x86_64::VirtAddr;

use libxernel::sync::Spinlock;

use super::context::thread_trampoline;
use super::context::{Context, TrapFrame};
use super::process::{Process, KERNEL_PROCESS};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
/// Current status of the thread
pub enum ThreadStatus {
    Initial,
    Running,
    Ready,
    Sleeping,
    BlockingOnIo,
    // TODO: better name
    Done,
}

#[derive(Debug, Clone, Copy)]
/// Priority level of the thread
pub enum ThreadPriority {
    Low,
    Normal,
    High,
}

impl ThreadPriority {
    /// Get the number of ms the thread can run from the priority
    pub fn ms(&self) -> u64 {
        match *self {
            Self::Low => 20,
            Self::Normal => 35,
            Self::High => 50,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct KernelStack {
    pub user_space_stack: usize,
    pub kernel_stack_top: usize,
}

fn idle_thread_fn() {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub struct Thread {
    pub id: usize,
    pub process: Weak<Spinlock<Process>>,
    pub status: Cell<ThreadStatus>,
    pub priority: ThreadPriority,
    pub context: UnsafeCell<*mut Context>,
    pub trap_frame: UnsafeCell<*mut TrapFrame>,
    // pub affinity
    pub thread_stack: usize,
    /// Only a user space thread has a kernel stack
    pub kernel_stack: Option<Pin<Box<KernelStack>>>,
}

impl Thread {
    pub fn new_kernel_thread(entry_point: VirtAddr) -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        let mut trap_frame = TrapFrame::new();

        trap_frame.ss = 0x10; // kernel stack segment
        trap_frame.cs = 0x8; // kernel code segment
        trap_frame.rip = entry_point.as_u64();
        trap_frame.rsp = thread_stack as u64;
        trap_frame.rflags = 0x202;

        let trap = UnsafeCell::new(Box::into_raw(Box::new(trap_frame)));

        let mut context = Context::new();

        context.rip = thread_trampoline as u64;
        unsafe {
            context.rbx = *trap.get() as u64;
        }

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: Cell::new(ThreadStatus::Initial),
            priority: ThreadPriority::Normal,
            context: UnsafeCell::new(Box::into_raw(Box::new(context))),
            trap_frame: trap,
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn kernel_thread_from_fn(entry: fn()) -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        println!("{:x}", thread_stack);

        let mut trap_frame = TrapFrame::new();

        trap_frame.ss = 0x10; // kernel stack segment
        trap_frame.cs = 0x8; // kernel code segment
        trap_frame.rip = entry as u64;
        trap_frame.rsp = thread_stack as u64;
        trap_frame.rflags = 0x202;

        println!("{:?}", trap_frame);

        let mut context = Context::new();

        context.rip = thread_trampoline as u64;

        let (trap_ptr, ctx_ptr) = unsafe {
            Thread::setup_stack_frame(thread_stack, trap_frame, context)
        };

        context.rbx = trap_ptr as u64;

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: Cell::new(ThreadStatus::Initial),
            priority: ThreadPriority::Normal,
            trap_frame: UnsafeCell::new(trap_ptr),
            context: UnsafeCell::new(ctx_ptr),
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn new_user_thread(parent_process: Arc<Spinlock<Process>>, entry_point: VirtAddr) -> Self {
        let thread_stack = parent_process.lock().new_user_stack();
        let kernel_stack_end = parent_process.lock().new_kernel_stack();

        let mut trap_frame = TrapFrame::new();

        trap_frame.ss = 0x2b; // user stack segment
        trap_frame.cs = 0x33; // user code segment
        trap_frame.rip = entry_point.as_u64();
        trap_frame.rsp = thread_stack as u64;
        trap_frame.rflags = 0x202;

        let mut context = Context::new();

        context.rip = thread_trampoline as u64;

        let mut parent = parent_process.lock();

        let (trap_ptr, ctx_ptr) = unsafe {
            Thread::setup_stack_frame(thread_stack, trap_frame, context)
        };

        Self {
            id: parent.next_tid(),
            thread_stack,
            process: Arc::downgrade(&parent_process),
            status: Cell::new(ThreadStatus::Initial),
            priority: ThreadPriority::Normal,
            trap_frame: UnsafeCell::new(Box::into_raw(Box::new(trap_frame))),
            context: UnsafeCell::new(core::ptr::null_mut()),
            kernel_stack: Some(Box::pin(KernelStack {
                user_space_stack: 0,
                kernel_stack_top: kernel_stack_end,
            })),
        }
    }

    unsafe fn setup_stack_frame(stack: usize, trap_frame: TrapFrame, ctx: Context) -> (*mut TrapFrame, *mut Context) {

        let ptr = stack as *mut u64;
        debug!("{:?}", ptr);

        //let ptr = ptr.offset(-1);

        ptr.write(trap_frame.ss);
        println!("first write");
        ptr.offset(-1).write(trap_frame.rsp);
        ptr.offset(-2).write(trap_frame.rflags);
        ptr.offset(-3).write(trap_frame.cs);
        ptr.offset(-4).write(trap_frame.rip);
        ptr.offset(-5).write(trap_frame.error_code);
        ptr.offset(-6).write(trap_frame.r15);
        ptr.offset(-7).write(trap_frame.r14);
        ptr.offset(-8).write(trap_frame.r13);
        ptr.offset(-9).write(trap_frame.r12);
        ptr.offset(-10).write(trap_frame.r11);
        ptr.offset(-11).write(trap_frame.r10);
        ptr.offset(-12).write(trap_frame.r9);
        ptr.offset(-13).write(trap_frame.r8);
        ptr.offset(-14).write(trap_frame.rdi);
        ptr.offset(-15).write(trap_frame.rsi);
        ptr.offset(-16).write(trap_frame.rdx);
        ptr.offset(-17).write(trap_frame.rcx);
        ptr.offset(-18).write(trap_frame.rbx);
        ptr.offset(-19).write(trap_frame.rax);
        ptr.offset(-20).write(trap_frame.rbp);

        ptr.offset(-21).write(ctx.rip);
        ptr.offset(-22).write(ctx.r15);
        ptr.offset(-23).write(ctx.r14);
        ptr.offset(-24).write(ctx.r13);
        ptr.offset(-25).write(ctx.r12);
        ptr.offset(-26).write(ctx.rbp);
        ptr.offset(-27).write(ptr.offset(-20) as u64);

        println!("{:?}", ptr.offset(-20) as *mut TrapFrame);

        (ptr.offset(-20) as *mut TrapFrame, ptr.offset(-27) as *mut Context) 
    }

    pub fn new_idle_thread() -> Self {
        // TODO: don't use a normal kernel task as a huge stack is allocated
        let mut thread = Self::kernel_thread_from_fn(idle_thread_fn);

        thread.priority = ThreadPriority::Low;

        thread
    }

    pub fn idle_thread() -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        let mut parent = KERNEL_PROCESS.lock();

        Self {
            id: parent.next_tid(),
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: Cell::new(ThreadStatus::Ready),
            priority: ThreadPriority::Low,
            context: UnsafeCell::new(core::ptr::null_mut()),
            trap_frame: UnsafeCell::new(core::ptr::null_mut()),
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn set_priority(&mut self, priority: ThreadPriority) {
        self.priority = priority;
    }

    pub fn is_kernel_thread(&self) -> bool {
        unsafe {
            let trap_frame_ptr = self.trap_frame.get();
            if !trap_frame_ptr.is_null() {
                let trap_frame_ref = *trap_frame_ptr;
                (*trap_frame_ref).cs == 0x8 && (*trap_frame_ref).ss == 0x10
            } else {
                false
            }
        }
    }

    pub fn get_process(&self) -> Option<Arc<Spinlock<Process>>> {
        self.process.upgrade()
    }
}
