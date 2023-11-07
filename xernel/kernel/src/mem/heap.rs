use core::alloc::GlobalAlloc;
use core::ptr::NonNull;

use libxernel::sync::Spinlock;
use linked_list_allocator::Heap;
use x86_64::structures::paging::{Page, PageSize, PageTableFlags, PhysFrame, Size2MiB};
use x86_64::VirtAddr;

use crate::{allocator::align_up, info};

use super::HEAP_START_ADDR;
use super::{frame::FRAME_ALLOCATOR, paging::KERNEL_PAGE_MAPPER};

// TODO: Replace heap by Buddy Allocator
static HEAP: Spinlock<Heap> = Spinlock::new(Heap::empty());

const HEAP_INITIAL_PAGE_COUNT: u64 = 2; // 4 MiB

struct Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut heap = HEAP.lock();

        if let Ok(ptr) = heap.allocate_first_fit(layout) {
            ptr.as_ptr()
        } else {
            // expand heap
            let expansion_size = align_up(layout.size(), Size2MiB::SIZE as usize);

            info!("expanding heap by {} MiB", expansion_size / 1024 / 1024);

            let current_top = align_up(heap.top() as usize, Size2MiB::SIZE as usize);

            for start_address in (current_top..current_top + expansion_size).step_by(Size2MiB::SIZE as usize) {
                let page = {
                    let mut allocator = FRAME_ALLOCATOR.lock();
                    allocator.allocate_frame::<Size2MiB>().unwrap()
                };

                KERNEL_PAGE_MAPPER.lock().map::<Size2MiB>(
                    PhysFrame::containing_address(page.start_address()),
                    Page::containing_address(VirtAddr::new(start_address as u64)),
                    PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE | PageTableFlags::PRESENT,
                    true,
                );

                heap.extend(Size2MiB::SIZE as usize);
            }

            // try to allocate again
            heap.allocate_first_fit(layout)
                .expect("heap allocation failed after expansion")
                .as_ptr()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if ptr.is_null() {
            return;
        }

        let mut heap = HEAP.lock();

        heap.deallocate(NonNull::new(ptr).unwrap(), layout);
    }
}

pub fn init() {
    let mut heap = HEAP.lock();
    let mut page_mapper = KERNEL_PAGE_MAPPER.lock();

    for start_address in (HEAP_START_ADDR..HEAP_START_ADDR + (HEAP_INITIAL_PAGE_COUNT * Size2MiB::SIZE) as usize)
        .step_by(Size2MiB::SIZE as usize)
    {
        let page = {
            let mut allocator = FRAME_ALLOCATOR.lock();
            allocator.allocate_frame::<Size2MiB>().unwrap()
        };

        page_mapper.map::<Size2MiB>(
            PhysFrame::containing_address(page.start_address()),
            Page::containing_address(VirtAddr::new(start_address as u64)),
            PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE | PageTableFlags::PRESENT,
            true,
        );
    }

    unsafe {
        heap.init(
            HEAP_START_ADDR as *mut u8,
            (HEAP_INITIAL_PAGE_COUNT * Size2MiB::SIZE) as usize,
        );
    }
}
