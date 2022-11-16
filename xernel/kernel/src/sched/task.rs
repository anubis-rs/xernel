use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::rc::Weak;
use alloc::vec::Vec;
use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;

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
    pub fn new_kernel_task(entry_point: VirtAddr, rsp: VirtAddr, rflags: u64) -> Self {
        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u64,
            page_table: None,
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: TaskContext::new(entry_point, rsp, rflags),
            is_kernel_task: true,
        }
    }

    pub fn new_user_task(entry_point: VirtAddr, rsp: VirtAddr, rflags: u64) -> Self {
        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u64,
            page_table: Some(PageTable::new()),
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: TaskContext::new(entry_point, rsp, rflags),
            is_kernel_task: false,
        }
    }

    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.priority = priority;
    }
}
