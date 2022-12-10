use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::alloc::alloc_zeroed;
use alloc::rc::Weak;
use alloc::vec::Vec;
use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;

use crate::mem::STACK_SIZE;
use crate::sched::context::TaskContext;

/// Ongoing counter for the TaskID
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
/// Current status of the task
pub enum TaskStatus {
    Running,
    Waiting,
    Sleeping,
    Zombie,
}

#[derive(Debug, Clone)]
/// Priority level of the task
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
}

impl Task {
    pub fn new_kernel_task(entry_point: VirtAddr) -> Self {
        let task_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = TaskContext::new();

        ctx.ss = 0x10; // kernel stack segment
        ctx.cs = 0x8; // kernel code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x202;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u64,
            page_table: None,
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
        }
    }

    pub fn kernel_task_from_fn(entry: fn()) -> Self {
        let task_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = TaskContext::new();

        ctx.ss = 0x10; // kernel stack segment
        ctx.cs = 0x8; // kernel code segment
        ctx.rip = entry as u64;
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x202;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u64,
            page_table: None,
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
        }
    }

    pub fn new_user_task(entry_point: VirtAddr) -> Self {
        // TODO: Alloc user stack via vmm, don't use kernel heap

        let task_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(4096, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let mut ctx = TaskContext::new();

        // TODO: Check if data segment has to be set too, currently setting stack segment to data
        ctx.ss = 0x33; // user stack segment
        ctx.cs = 0x2b; // user code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x200;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u64,
            page_table: Some(PageTable::new()),
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
        }
    }

    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.priority = priority;
    }

    pub fn is_kernel_task(&self) -> bool {
        self.context.cs == 0x8 && self.context.ss == 0x10
    }
}
