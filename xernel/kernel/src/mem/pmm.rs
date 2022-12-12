use core::ptr::NonNull;

use crate::allocator::buddy::BuddyAllocator;
use libxernel::sync::{Once, Spinlock};
use limine::{LimineMemmapEntry, LimineMemmapRequest, LimineMemoryMapEntryType, NonNullPtr};
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    PhysAddr,
};

static MMAP_REQUEST: LimineMemmapRequest = LimineMemmapRequest::new(0);

pub static MEMORY_MAP: Once<&'static [NonNullPtr<LimineMemmapEntry>]> = Once::new();

pub static FRAME_ALLOCATOR: Spinlock<BuddyAllocator> = Spinlock::new(BuddyAllocator::new());

// FIXME: Check results accordingly and do not assume unwrap will work
unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB> for BuddyAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.allocate(0);
        let start_addr = frame.unwrap().as_ptr() as u64;
        let pframe = PhysFrame::from_start_address(PhysAddr::new(start_addr));
        Some(pframe.unwrap())
    }
}

impl x86_64::structures::paging::FrameDeallocator<Size4KiB> for BuddyAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.deallocate(
            NonNull::new(frame.start_address().as_u64() as *mut u8).unwrap(),
            0,
        )
        .unwrap();
    }
}

pub fn init() {
    let mut buddy = FRAME_ALLOCATOR.lock();

    MEMORY_MAP.set_once(
        MMAP_REQUEST
            .get_response()
            .get()
            .expect("barebones: recieved no mmap")
            .memmap(),
    );

    for entry in *MEMORY_MAP {
        if entry.typ == LimineMemoryMapEntryType::Usable {
            unsafe {
                // FIXME: Check result of add_region function
                // FIXME: Last add_region returns NullPointer in buddy_of function
                buddy.add_region(
                    NonNull::new(entry.base as *mut u8).unwrap(),
                    NonNull::new((entry.base + entry.len) as *mut u8).unwrap(),
                );
            }
        }
    }

    dbg!("{}", buddy.stats);
}
