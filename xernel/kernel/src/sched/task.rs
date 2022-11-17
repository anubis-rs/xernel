use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::alloc::alloc_zeroed;
use alloc::rc::Weak;
use alloc::vec::Vec;
use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;

use crate::mem::STACK_SIZE;
use crate::sched::context::TaskContext;

static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Waiting,
    Sleeping,
    Zombie,
}

#[derive(Debug, Clone)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub page_table: Option<PageTable>,
    pub parent: Weak<Task>,
    pub children: Vec<Task>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub context: TaskContext,
    pub is_kernel_task: bool,
}

impl Task {
    pub fn new_kernel_task(entry_point: VirtAddr) -> Self {
        /*  FIXME: Only allocate kernel stacks on the kernel heap
                    Write function for vmm to allocate stack for user land programs (stack, heap, etc.)
        */
        let task_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = TaskContext::new();

        ctx.ss = 0x10;
        ctx.cs = 0x8;
        ctx.rip = entry_point.as_u64();
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x202;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u64,
            page_table: None,
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
            is_kernel_task: true,
        }
    }

    pub fn new_user_task(entry_point: VirtAddr) -> Self {
        let task_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(4096, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = TaskContext::new();

        //ctx.ss = 0x10;
        //ctx.cs = 0x8;
        ctx.rip = entry_point.as_u64();
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x200;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u64,
            page_table: Some(PageTable::new()),
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
            is_kernel_task: false,
        }
    }

    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.priority = priority;
    }
}
