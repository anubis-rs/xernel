use alloc::sync::Weak;
use core::sync::atomic::{AtomicUsize, Ordering};
use libxernel::syscall::{MapFlags, ProtectionFlags};
use x86_64::structures::paging::{Page, PageSize, PageTableFlags, Size4KiB};
use x86_64::{align_down, align_up, VirtAddr};

use crate::fs::file::File;
use crate::fs::vnode::VNode;
use crate::mem::frame::FRAME_ALLOCATOR;
use crate::mem::vm::{protflags_from_ptflags, Vm};
use crate::mem::{HIGHER_HALF_OFFSET, KERNEL_THREAD_STACK_TOP, PROCESS_START, STACK_SIZE};
use crate::VFS;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use libxernel::sync::{Once, Spinlock};

use crate::mem::paging::{Pagemap, KERNEL_PAGE_MAPPER};
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
    pub fds: BTreeMap<usize, File>,
    pub kernel_thread_stack_top: usize,
    pub thread_id_counter: usize,
    pub vm: Vm,
    pub cwd: Arc<Spinlock<VNode>>,
}

impl Process {
    pub fn new(parent_process: Option<Arc<Spinlock<Process>>>) -> Self {
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
            kernel_thread_stack_top: KERNEL_THREAD_STACK_TOP as usize,
            thread_id_counter: 0,
            vm: Vm::new(),
            cwd: VFS.lock().root_node(),
        }
    }

    pub fn new_kernel_stack(&mut self) -> usize {
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

    pub fn new_user_stack(&mut self) -> usize {
        let stack_bottom = self
            .vm
            .create_entry_high(
                STACK_SIZE as usize,
                ProtectionFlags::READ | ProtectionFlags::WRITE,
                MapFlags::ANONYMOUS,
            )
            .as_u64() as usize;
        let stack_top = STACK_SIZE as usize + stack_bottom;

        for addr in (stack_bottom..stack_top).step_by(Size4KiB::SIZE as usize) {
            let phys_page = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();
            let virt_page = Page::from_start_address(VirtAddr::new(addr as u64)).unwrap();

            self.page_table.as_mut().unwrap().map(
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

    /// Load an ELF file into the process memory
    ///
    /// Returns the entry point of the ELF file
    pub fn load_elf(&mut self, elf_data: &[u8]) -> VirtAddr {
        let elf = elf::ElfBytes::<elf::endian::NativeEndian>::minimal_parse(elf_data).expect("Failed to parse ELF");

        for ph in elf.segments().expect("Failed to get program headers") {
            if ph.p_type == elf::abi::PT_LOAD {
                let start = ph.p_vaddr + PROCESS_START;
                let end = start + ph.p_memsz;

                let page_start = align_down(start, Size4KiB::SIZE);
                let page_end = align_up(end, Size4KiB::SIZE);

                let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;

                if ph.p_flags & elf::abi::PF_X == 0 {
                    // flags |= PageTableFlags::NO_EXECUTE; TODO: fix NO_EXECUTE in page mapper
                }
                if ph.p_flags & elf::abi::PF_W != 0 {
                    flags |= PageTableFlags::WRITABLE;
                }

                for addr in (page_start..page_end).step_by(Size4KiB::SIZE as usize) {
                    let phys_page = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();
                    let virt_page = Page::from_start_address(VirtAddr::new(addr)).unwrap();

                    self.page_table
                        .as_mut()
                        .unwrap()
                        .map(phys_page, virt_page, flags, false);

                    // write data to the page
                    let page_offset = if addr.overflowing_sub(start).1 { start - addr } else { 0 };
                    let data_len = Size4KiB::SIZE - page_offset;
                    let segment_offset: u64 = addr + page_offset - start;

                    let data = &elf_data
                        [(ph.p_offset + segment_offset) as usize..(ph.p_offset + segment_offset + data_len) as usize];

                    unsafe {
                        core::ptr::copy(
                            data.as_ptr(),
                            (phys_page.start_address().as_u64() + page_offset + *HIGHER_HALF_OFFSET) as *mut u8,
                            data_len as usize,
                        );
                    }
                }

                self.vm.create_entry_at(
                    VirtAddr::new(page_start),
                    (page_end - page_start) as usize,
                    protflags_from_ptflags(flags),
                    MapFlags::ANONYMOUS,
                );
            }
        }

        VirtAddr::new(elf.ehdr.e_entry + PROCESS_START)
    }

    pub fn next_tid(&mut self) -> usize {
        let tid = self.thread_id_counter;
        self.thread_id_counter += 1;

        tid
    }

    pub fn append_fd(&mut self, file_handle: File) -> u32 {
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

    pub fn get_filehandle_from_fd(&self, fd: usize) -> &File {
        let handle = self.fds.get(&fd).expect("Failed to get FileHandle for fd");

        handle
    }

    pub fn get_page_table(&mut self) -> &mut Option<Pagemap> {
        &mut self.page_table
    }

    pub fn vm(&mut self) -> &mut Vm {
        &mut self.vm
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.vm.clean_up();
    }
}
