use alloc::rc::Weak;
use alloc::vec::Vec;
use x86_64::structures::paging::PageTable;
use x86_64::VirtAddr;

pub enum TaskStatus {
    Running,
    Waiting,
    Sleeping,
    Zombie,
}

pub enum TaskPriority {
    Low,
    Normal,
    High,
}

pub struct TaskContext {
    // todo: add registers
}

pub struct Task {
    pub id: u64,
    // Maybe we should use a pointer to the page table instead of the page table itself
    pub page_table: Option<PageTable>,
    pub parent: Weak<Task>,
    pub children: Vec<Task>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub stack: *mut u8,
    pub stack_size: usize,
    pub entry_point: VirtAddr,
    pub context: TaskContext,
}
