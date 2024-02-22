use alloc::sync::Arc;
use elf::abi::{ET_EXEC, PT_LOAD};
use elf::endian::LittleEndian;
use elf::ElfBytes;
use libxernel::sync::Spinlock;
use x86_64::structures::paging::{Page, PageSize, PageTableFlags, PhysFrame, Size4KiB};

use crate::allocator::align_up;
use crate::mem::frame::FRAME_ALLOCATOR;
use crate::mem::paging::KERNEL_PAGE_MAPPER;
use crate::mem::STACK_SIZE;

use super::context::CpuContext;
use super::process::{Process, KERNEL_PROCESS};

use core::pin::Pin;
use core::ptr::write_bytes;

use alloc::boxed::Box;
use alloc::sync::Weak;
use x86_64::VirtAddr;

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
#[repr(C, packed)]
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
    pub context: CpuContext,
    pub thread_stack: usize,
    /// Only a user space thread has a kernel stack
    pub kernel_stack: Option<Pin<Box<KernelStack>>>,
}

impl Thread {
    pub fn new_kernel_thread(entry_point: VirtAddr) -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        let mut ctx = CpuContext::new();

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
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn kernel_thread_from_fn(entry: fn()) -> Self {
        let thread_stack = KERNEL_PROCESS.lock().new_kernel_stack();

        let mut ctx = CpuContext::new();

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
            thread_stack,
            kernel_stack: None,
        }
    }

    pub fn new_user_thread(parent_process: Arc<Spinlock<Process>>, entry_point: VirtAddr) -> Self {
        let thread_stack = parent_process.lock().new_user_stack();
        let kernel_stack_end = parent_process.lock().new_kernel_stack();

        let mut ctx = CpuContext::new();

        ctx.ss = 0x2b; // user stack segment
        ctx.cs = 0x33; // user code segment
        ctx.rip = entry_point.as_u64();
        ctx.rsp = thread_stack as u64;
        ctx.rflags = 0x202;

        let mut parent = parent_process.lock();

        Self {
            id: parent.next_tid(),
            thread_stack,
            process: Arc::downgrade(&parent_process),
            status: ThreadStatus::Ready,
            priority: ThreadPriority::Normal,
            context: ctx,
            kernel_stack: Some(Box::pin(KernelStack {
                user_space_stack: 0,
                kernel_stack_top: kernel_stack_end,
            })),
        }
    }

    pub fn new_user_thread_from_elf(parent_process: Arc<Spinlock<Process>>, elf_data: &[u8]) -> Self {
        let thread_stack = parent_process.lock().new_user_stack();
        let kernel_stack_end = parent_process.lock().new_kernel_stack();

        let mut ctx = CpuContext::new();

        ctx.ss = 0x2b; // user stack segment
        ctx.cs = 0x33; // user code segment
        ctx.rsp = thread_stack as u64;
        ctx.rflags = 0x202;

        let mut parent = parent_process.lock();

        let mut thread = Self {
            id: parent.next_tid(),
            thread_stack,
            process: Arc::downgrade(&parent_process),
            status: ThreadStatus::Ready,
            priority: ThreadPriority::Normal,
            context: ctx,
            kernel_stack: Some(Box::pin(KernelStack {
                user_space_stack: 0,
                kernel_stack_top: kernel_stack_end,
            })),
        };

        parent.unlock();

        thread.load_elf(elf_data);

        thread
    }

    fn load_elf(&mut self, elf_data: &[u8]) {
        let file = ElfBytes::<LittleEndian>::minimal_parse(elf_data).expect("opening elf file failed");

        self.context.rip = file.ehdr.e_entry;

        if file.ehdr.e_type != ET_EXEC {
            panic!("elf: not an executable");
        }

        let segments = file.segments().expect("parsing segments failed");

        for segment in segments {
            if segment.p_type == PT_LOAD {
                dbg!("{:#x?}", segment);
                let file_size = segment.p_filesz as usize;
                let mem_size = segment.p_memsz as usize;
                let vaddr = segment.p_vaddr as usize;

                assert!(file_size <= mem_size);
                assert!(vaddr % Size4KiB::SIZE as usize == 0);

                let mem_size = align_up(mem_size, Size4KiB::SIZE as usize);

                // TODO: add mappings to Vm
                for page in (vaddr..vaddr + mem_size).step_by(Size4KiB::SIZE as usize) {
                    let page = Page::<Size4KiB>::from_start_address(VirtAddr::new(page as u64)).unwrap();
                    let frame = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();

                    let process = self.get_process().unwrap();
                    let process = process.lock();

                    dbg!("before mapping");
                    process.get_page_table().unwrap().map(
                        frame,
                        page,
                        Self::elf_flags_to_pt_flags(segment.p_flags)
                            | PageTableFlags::PRESENT
                            | PageTableFlags::USER_ACCESSIBLE
                            | PageTableFlags::WRITABLE, // TODO: make this depend on the elf flags, currently we need it to write the content of the segment ==> make it read-only after writing
                        true,
                    );
                    dbg!("after mapping");

                    unsafe {
                        write_bytes(page.start_address().as_mut_ptr::<u8>(), 0, Size4KiB::SIZE as usize);
                    }
                }

                unsafe {
                    core::ptr::copy_nonoverlapping(
                        elf_data.as_ptr().add(segment.p_offset as usize),
                        vaddr as *mut u8,
                        file_size,
                    );
                }

                dbg!("segment loaded");
            }
        }
    }

    pub fn elf_flags_to_pt_flags(flags: u32) -> PageTableFlags {
        let mut ret = PageTableFlags::empty();

        if flags & 1 == 0 {
            ret |= PageTableFlags::NO_EXECUTE;
        }

        if flags & 2 == 2 {
            ret |= PageTableFlags::WRITABLE;
        }

        ret
    }

    pub fn new_idle_thread() -> Self {
        // TODO: don't use a normal kernel task as a huge stack is allocated
        let mut thread = Self::kernel_thread_from_fn(idle_thread_fn);

        thread.priority = ThreadPriority::Low;

        thread
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

impl Drop for Thread {
    fn drop(&mut self) {
        if self.is_kernel_thread() {
            let mut page_mapper = KERNEL_PAGE_MAPPER.lock();
            let mut frame_allocator = FRAME_ALLOCATOR.lock();

            for addr in (self.thread_stack..self.thread_stack + STACK_SIZE as usize).step_by(Size4KiB::SIZE as usize) {
                unsafe {
                    let page = Page::<Size4KiB>::from_start_address(VirtAddr::new(addr as u64)).unwrap();
                    let phys_addr = page_mapper.translate(page.start_address()).unwrap();

                    frame_allocator.deallocate_frame(PhysFrame::<Size4KiB>::containing_address(phys_addr));
                    page_mapper.unmap(page.start_address());
                }
            }
        }
    }
}
