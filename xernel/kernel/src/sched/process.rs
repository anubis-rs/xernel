use alloc::sync::Weak;
use core::sync::atomic::{AtomicUsize, Ordering};
use x86_64::structures::paging::{Page, PageSize, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

use crate::fs::file::FileHandle;
use crate::mem::pmm::FRAME_ALLOCATOR;
use crate::mem::{KERNEL_THREAD_STACK_TOP, STACK_SIZE, USER_THREAD_STACK_TOP};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use libxernel::sync::{Once, Spinlock};

use crate::mem::vmm::{Pagemap, KERNEL_PAGE_MAPPER};
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
    pub kernel_thread_stack_top: usize,
    pub user_thread_stack_top: usize,
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

        Self {
            pid: PROCESS_ID_COUNTER.fetch_add(1, Ordering::AcqRel),
            page_table: Some(page_map),
            parent,
            children: Vec::new(),
            threads: Vec::new(),
            fds: BTreeMap::new(),
            is_kernel_process,
            kernel_thread_stack_top: KERNEL_THREAD_STACK_TOP as usize,
            user_thread_stack_top: USER_THREAD_STACK_TOP as usize,
            thread_id_counter: 0,
        }
    }

    pub fn new_kernel_thread_stack(&mut self) -> usize {
        let stack_top = self.kernel_thread_stack_top;
        self.kernel_thread_stack_top -= STACK_SIZE as usize;
        let stack_bottom = self.kernel_thread_stack_top;

        // create guard page
        self.kernel_thread_stack_top -= Size4KiB::SIZE as usize;

        for addr in (stack_bottom..stack_top).step_by(Size4KiB::SIZE as usize) {
            let phys_page = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();
            let virt_page = Page::from_start_address(VirtAddr::new(addr as u64)).unwrap();

            KERNEL_PAGE_MAPPER.lock().map(
                phys_page,
                virt_page,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                true,
            );
        }

        stack_top
    }

    pub fn new_user_thread_stack(&mut self) -> usize {
        let stack_top = self.user_thread_stack_top;
        self.user_thread_stack_top -= STACK_SIZE as usize;
        let stack_bottom = self.user_thread_stack_top;

        // create guard page
        self.user_thread_stack_top -= Size4KiB::SIZE as usize;

        for addr in (stack_bottom..stack_top).step_by(Size4KiB::SIZE as usize) {
            let phys_page = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();
            let virt_page = Page::from_start_address(VirtAddr::new(addr as u64)).unwrap();

            self.page_table.clone().unwrap().map(
                phys_page,
                virt_page,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::NO_EXECUTE,
                false,
            );
        }

        stack_top
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
