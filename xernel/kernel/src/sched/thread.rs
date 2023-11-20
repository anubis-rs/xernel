use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::sync::Weak;
use core::cell::{Cell, UnsafeCell};
use core::pin::Pin;

use x86_64::VirtAddr;

use libxernel::sync::Spinlock;

use super::context::{Context, TrapFrame};
use super::process::{KERNEL_PROCESS, Process};

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

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: Cell::new(ThreadStatus::Initial),
            priority: ThreadPriority::Normal,
            context: UnsafeCell::new(core::ptr::null_mut()),
            trap_frame: UnsafeCell::new(Box::into_raw(Box::new(trap_frame))),
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn kernel_thread_from_fn(entry: fn()) -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        let mut trap_frame = TrapFrame::new();

        trap_frame.ss = 0x10; // kernel stack segment
        trap_frame.cs = 0x8; // kernel code segment
        trap_frame.rip = entry as u64;
        trap_frame.rsp = thread_stack as u64;
        trap_frame.rflags = 0x202;

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: Cell::new(ThreadStatus::Initial),
            priority: ThreadPriority::Normal,
            trap_frame: UnsafeCell::new(Box::into_raw(Box::new(trap_frame))),
            context: UnsafeCell::new(core::ptr::null_mut()),
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

        let mut parent = parent_process.lock();

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
