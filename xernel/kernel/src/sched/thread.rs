use alloc::sync::Arc;
use libxernel::sync::Spinlock;

use super::context::ThreadContext;
use super::process::{Process, KERNEL_PROCESS};

use core::alloc::Layout;
use core::pin::Pin;

use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::sync::Weak;
use x86_64::VirtAddr;

use crate::mem::vmm::Pagemap;
use crate::mem::STACK_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Current status of the thread
pub enum ThreadStatus {
    Running,
    Ready,
    Sleeping,
    BlockingOnIo, // TODO: better name
                  // Zombie,
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
#[repr(packed)]
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
    pub status: ThreadStatus,
    pub priority: ThreadPriority,
    pub context: ThreadContext,
    pub thread_stack: usize,
    /// Only a user space thread has a kernel stack
    pub kernel_stack: Option<Pin<Box<KernelStack>>>,
}

impl Thread {
    pub fn new_kernel_thread(entry_point: VirtAddr) -> Self {
        let thread_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = ThreadContext::new();

        ctx.ss = 0x10; // kernel stack segment
        ctx.cs = 0x8; // kernel code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = thread_stack as u64;
        ctx.rflags = 0x202;

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: ThreadStatus::Ready,
            priority: ThreadPriority::Normal,
            context: ctx,
            thread_stack: thread_stack as usize,
            kernel_stack: None,
        }
    }

    pub fn kernel_thread_from_fn(entry: fn()) -> Self {
        let thread_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = ThreadContext::new();

        ctx.ss = 0x10; // kernel stack segment
        ctx.cs = 0x8; // kernel code segment
        ctx.rip = entry as u64;
        ctx.rsp = thread_stack as u64;
        ctx.rflags = 0x202;

        let mut parent = KERNEL_PROCESS.lock();

        let tid = parent.next_tid();

        Self {
            id: tid,
            process: Arc::downgrade(&KERNEL_PROCESS),
            status: ThreadStatus::Ready,
            priority: ThreadPriority::Normal,
            context: ctx,
            thread_stack: thread_stack as usize,
            kernel_stack: None,
        }
    }

    pub fn new_user_thread(parent_process: Arc<Spinlock<Process>>, entry_point: VirtAddr) -> Self {
        // TODO: Alloc user stack via vmm, don't use kernel heap

        let thread_stack = {
            //let layout = Layout::from_size_align_unchecked(4096, 0x1000);
            //alloc_zeroed(layout).add(layout.size());
            (entry_point.as_u64() + 2 * 1024 * 1024) as *mut u8 // FIXME: hardcoded stack size somewhere after start address!!!!!!
        };

        // TODO: don't allocate kernel stack on the heap as their is no protection against overflows
        let (_, kernel_stack_end) = unsafe {
            const STACK_SIZE: u64 = 128 * 1024; // TODO: figure out which stack size is needed
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            let ptr = alloc_zeroed(layout).add(layout.size());
            (ptr as u64, ptr as u64 + STACK_SIZE)
        };

        let mut page_map = Pagemap::new(None);
        page_map.fill_with_kernel_entries();
        // TODO: access page map of parent

        let mut ctx = ThreadContext::new();

        ctx.ss = 0x2b; // user stack segment
        ctx.cs = 0x33; // user code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = thread_stack as u64;
        ctx.rflags = 0x202;

        let mut parent = parent_process.lock();

        Self {
            id: parent.next_tid(),
            thread_stack: thread_stack as usize,
            process: Arc::downgrade(&parent_process),
            status: ThreadStatus::Ready,
            priority: ThreadPriority::Normal,
            context: ctx,
            kernel_stack: Some(Box::pin(KernelStack {
                user_space_stack: 0,
                kernel_stack_top: kernel_stack_end as usize,
            })),
        }
    }

    pub fn new_idle_thread() -> Self {
        todo!("idle thread");
        // TODO: don't use a normal kernel task as a huge stack is allocated
        //let mut task = Self::kernel_task_from_fn(idle_task_fn);

        //task.priority = TaskPriority::Low;

        //task
    }

    pub fn set_priority(&mut self, priority: ThreadPriority) {
        self.priority = priority;
    }

    pub fn is_kernel_thread(&self) -> bool {
        self.context.cs == 0x8 && self.context.ss == 0x10
    }

    pub fn get_process(&self) -> Option<Arc<Spinlock<Process>>> {
        self.process.upgrade()
    }
}
