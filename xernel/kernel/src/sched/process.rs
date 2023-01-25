use alloc::sync::Weak;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::fs::file::FileHandle;
use crate::mem::{KERNEL_THREAD_STACK_TOP, USER_THREAD_STACK_TOP};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use libxernel::sync::{Once, Spinlock};

use crate::mem::vmm::Pagemap;
use crate::sched::thread::Thread;

/// Ongoing counter for the ProcessID
static PROCESS_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub static KERNEL_PROCESS: Once<Arc<Spinlock<Process>>> = Once::new();

pub struct Process {
    pub pid: usize,
    /// A kernel process has no page table
    pub page_table: Option<Pagemap>,
    pub parent: Weak<Spinlock<Process>>,
    pub children: Vec<Arc<Spinlock<Process>>>,
    pub threads: Vec<Arc<Spinlock<Thread>>>,
    pub fds: BTreeMap<usize, FileHandle>,
    pub is_kernel_process: bool,
    pub thread_stack_top: usize,
    pub thread_id_counter: usize,
    // TODO: add cwd here
    // TODO: list of memory maps (look at mmap)
}

impl Process {
    pub fn new(parent_process: Option<Arc<Spinlock<Process>>>, is_kernel_process: bool) -> Self {
        let mut page_map = Pagemap::new(None);
        page_map.fill_with_kernel_entries();

        let parent = match parent_process {
            Some(p) => Arc::downgrade(&p),
            None => Weak::new(),
        };

        let thread_stack_top = match is_kernel_process {
            true => KERNEL_THREAD_STACK_TOP,
            false => USER_THREAD_STACK_TOP,
        };

        Self {
            pid: PROCESS_ID_COUNTER.fetch_add(1, Ordering::AcqRel),
            page_table: Some(page_map),
            parent,
            children: Vec::new(),
            threads: Vec::new(),
            fds: BTreeMap::new(),
            is_kernel_process,
            thread_stack_top: thread_stack_top as usize,
            thread_id_counter: 0,
        }
    }

    pub fn next_tid(&mut self) -> usize {
        let tid = self.thread_id_counter;
        self.thread_id_counter += 1;

        tid
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

    pub fn get_page_table(&self) -> Option<Pagemap> {
        self.page_table.clone()
    }
}
