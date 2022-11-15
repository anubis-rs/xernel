pub enum TaskStatus {
    Running,
    Waiting,
    Sleeping,
    Zombie,
}

pub struct TaskContext {
    // todo: add registers
}

pub struct Task {
    pub id: u64,
    // Maybe we should use a pointer to the page table instead of the page table itself
    pub page_table: PageTable,
    pub parent: Option<&'static Task>,
    // Maybe we shouldn't use a 'static lifetime here for the children tasks
    pub children: Vec<&'static Task>,
    pub status: TaskStatus,
    // Make priority an enum?
    pub priority: u8,
    pub stack: *mut u8,
    pub stack_size: usize,
    pub entry_point: VirtAddr,
    pub context: TaskContext,
}
