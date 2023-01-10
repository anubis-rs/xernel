use core::alloc::Layout;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::fs::file::FileHandle;
use alloc::alloc::{alloc_zeroed, dealloc};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Weak;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::mem::vmm::Pagemap;
use crate::mem::STACK_SIZE;
use crate::sched::context::TaskContext;

/// Ongoing counter for the TaskID
static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
/// Current status of the task
pub enum TaskStatus {
    Running,
    Waiting,
    Sleeping,
    Zombie,
}

#[derive(Debug, Clone, Copy)]
/// Priority level of the task
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

impl TaskPriority {
    /// Get the number of ms the task can run from the priority
    pub fn ms(&self) -> u64 {
        match *self {
            Self::Low => 20,
            Self::Normal => 35,
            Self::High => 50,
        }
    }
}

/// The kernel stack should only use 4 kib pages as the Drop implementation depends on it
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct KernelStack {
    pub user_space_stack: u64,
    pub start: u64,
    pub end: u64,
}

pub struct Task {
    pub id: u64,
    page_table: Option<Pagemap>,
    pub parent: Weak<Task>,
    pub children: Vec<Task>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub context: TaskContext,
    pub fds: BTreeMap<usize, FileHandle>,
    pub kernel_stack: Option<Pin<Box<KernelStack>>>,
}

impl Drop for Task {
    fn drop(&mut self) {
        if let Some(stack) = &self.kernel_stack {
            unsafe {
                dealloc(
                    stack.start as *mut u8,
                    Layout::from_size_align(stack.end as usize - stack.start as usize, 1).unwrap(),
                );
            }
        }
    }
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
            fds: BTreeMap::new(),
            kernel_stack: None,
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
            fds: BTreeMap::new(),
            kernel_stack: None,
        }
    }

    pub fn new_user_task(entry_point: VirtAddr) -> Self {
        // TODO: Alloc user stack via vmm, don't use kernel heap

        let task_stack = {
            //let layout = Layout::from_size_align_unchecked(4096, 0x1000);
            //alloc_zeroed(layout).add(layout.size());
            (entry_point.as_u64() + 2 * 1024 * 1024) as *mut u8 // FIXME: hardcoded stack size somewhere after start address!!!!!!
        };

        // TODO: don't allocate kernel stack on the heap as their is no protection against overflows
        let (kernel_stack_start, kernel_stack_end) = unsafe {
            const STACK_SIZE: u64 = 128 * 1024; // TODO: figure out which stack size is needed
            let layout = Layout::from_size_align_unchecked(STACK_SIZE as usize, 0x1000);
            let ptr = alloc_zeroed(layout).add(layout.size());
            (ptr as u64, ptr as u64 + STACK_SIZE)
        };

        let mut page_map = Pagemap::new(None);
        page_map.fill_with_kernel_entries();

        let mut ctx = TaskContext::new();

        ctx.ss = 0x2b; // user stack segment
        ctx.cs = 0x33; // user code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = task_stack as u64;
        ctx.rflags = 0x202;

        Self {
            id: TASK_ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u64,
            page_table: Some(page_map),
            parent: Weak::new(),
            children: Vec::new(),
            status: TaskStatus::Waiting,
            priority: TaskPriority::Normal,
            context: ctx,
            fds: BTreeMap::new(),
            kernel_stack: Some(Box::pin(KernelStack {
                user_space_stack: 0,
                start: kernel_stack_start,
                end: kernel_stack_end,
            })),
        }
    }

    pub fn get_page_table(&self) -> Option<Pagemap> {
        if self.is_kernel_task() {
            return None;
        }

        if let Some(page_table) = &self.page_table {
            Some(page_table.clone())
        } else {
            // get pagetable of parent
            self.parent.upgrade().unwrap().get_page_table()
        }
    }

    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.priority = priority;
    }

    pub fn is_kernel_task(&self) -> bool {
        self.context.cs == 0x8 && self.context.ss == 0x10
    }

    pub fn append_fd(&mut self, file_handle: FileHandle) -> u32 {
        let mut counter = 0;

        let fd = loop {
            if let alloc::collections::btree_map::Entry::Vacant(e) = self.fds.entry(counter) {
                e.insert(file_handle);
                break counter;
            }

            counter += 1;
        };

        fd as u32
    }

    pub fn get_filehandle_from_fd(&self, fd: usize) -> &FileHandle {
        let handle = self.fds.get(&fd).expect("Failed to get FileHandle for fd");

        handle
    }
}
