use core::alloc::GlobalAlloc;
use core::ptr::NonNull;

use libxernel::sync::Spinlock;
use linked_list_allocator::Heap;
use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::VirtAddr;

use super::{pmm::FRAME_ALLOCATOR, vmm::KERNEL_PAGE_MAPPER, FRAME_SIZE};

// TODO: Replace heap by Buddy Allocator
static HEAP: Spinlock<Heap> = Spinlock::new(Heap::empty());

const HEAP_START_ADDR: usize = 0x0000_1000_0000_0000;

const HEAP_INITIAL_PAGE_COUNT: u64 = 1024; // 4 MiB

struct Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut heap = HEAP.lock();

        // TODO: check if the allocation fails and maybe increase the heap size to make it work
        heap.allocate_first_fit(layout)
            .expect("out of memory")
            .as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if ptr.is_null() {
            return;
        }

        let mut heap = HEAP.lock();

        heap.deallocate(NonNull::new(ptr).unwrap(), layout);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub fn init() {
    let mut heap = HEAP.lock();
    let mut page_mapper = KERNEL_PAGE_MAPPER.lock();

    // TODO: don't use 4kib pages
    for start_address in (HEAP_START_ADDR
        ..HEAP_START_ADDR + (HEAP_INITIAL_PAGE_COUNT * FRAME_SIZE) as usize)
        .step_by(FRAME_SIZE as usize)
    {
        let page = {
            let mut allocator = FRAME_ALLOCATOR.lock();
            allocator.allocate_frame::<Size4KiB>().unwrap()
        };

        page_mapper.map::<Size4KiB>(
            PhysFrame::containing_address(page.start_address()),
            Page::containing_address(VirtAddr::new(start_address as u64)),
            PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE | PageTableFlags::PRESENT,
            true,
        );
    }

    unsafe {
        heap.init(
            HEAP_START_ADDR as *mut u8,
            (HEAP_INITIAL_PAGE_COUNT * FRAME_SIZE) as usize,
        );
    }
}
