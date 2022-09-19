use core::alloc::GlobalAlloc;
use core::ptr::NonNull;

use libxernel::spin::Spinlock;
use linked_list_allocator::Heap;
use x86_64::structures::paging::{FrameAllocator, PageTableFlags};
use x86_64::VirtAddr;

use crate::{print, println};

use super::{
    pmm::{FRAME_ALLOCATOR, FRAME_SIZE},
    vmm::KERNEL_PAGE_MAPPER,
};

static HEAP: Spinlock<Heap> = Spinlock::new(Heap::empty());

const HEAP_START_ADDR: usize = 0x0000_1000_0000_0000;

const HEAP_INITIAL_PAGE_COUNT: u64 = 1024; // 4 MiB

struct Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        println!("requested layout: {:#?}", layout);

        let mut heap = HEAP.lock();

        // TODO: check if the allocation fails and maybe increase the heap size to make it work
        heap.allocate_first_fit(layout)
            .expect("out of memory")
            .as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        println!("deallocating layout: {:#?}", layout);

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

    for start_address in (HEAP_START_ADDR
        ..HEAP_START_ADDR + (HEAP_INITIAL_PAGE_COUNT * FRAME_SIZE) as usize)
        .step_by(FRAME_SIZE as usize)
    {
        dbg!("mapping heap frame at {:#x}", start_address);
        let page = {
            let mut allocator = FRAME_ALLOCATOR.lock();
            allocator.allocate_frame().unwrap()
        };
        unsafe {
            page_mapper
                .map(
                    page.start_address(),
                    VirtAddr::new(start_address as u64),
                    PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                    true,
                )
                .unwrap();
        }
    }

    dbg!("end init");

    unsafe {
        heap.init(
            HEAP_START_ADDR as *mut u8,
            (HEAP_INITIAL_PAGE_COUNT * FRAME_SIZE) as usize,
        );
    }
}
