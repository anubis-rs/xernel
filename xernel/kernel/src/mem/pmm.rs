use core::ptr::NonNull;

use crate::allocator::{buddy::BuddyAllocator, order_for_size};
use libxernel::sync::{Once, Spinlock};
use limine::{LimineMemmapEntry, LimineMemmapRequest, LimineMemoryMapEntryType, NonNullPtr};
use x86_64::{
    structures::paging::{PageSize, PhysFrame},
    PhysAddr,
};

static MMAP_REQUEST: LimineMemmapRequest = LimineMemmapRequest::new(0);

pub static MEMORY_MAP: Once<&'static [NonNullPtr<LimineMemmapEntry>]> = Once::new();

pub struct PhysFrameAllocator(BuddyAllocator);

pub static FRAME_ALLOCATOR: Spinlock<PhysFrameAllocator> = Spinlock::new(PhysFrameAllocator(BuddyAllocator::new()));

impl PhysFrameAllocator {
    pub fn allocate_frame<P: PageSize>(&mut self) -> Option<PhysFrame<P>> {
        let order = order_for_size(P::SIZE as usize);

        let frame = self.0.allocate(order);
        let start_addr = frame.unwrap().as_ptr() as u64;
        let pframe = PhysFrame::from_start_address(PhysAddr::new(start_addr));
        pframe.ok()
    }

    pub unsafe fn deallocate_frame<P: PageSize>(&mut self, frame: PhysFrame<P>) {
        let order = order_for_size(P::SIZE as usize);

        self.0.deallocate(
            NonNull::new(frame.start_address().as_u64() as *mut u8).unwrap(),
            order,
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
                buddy.0.add_region(
                    NonNull::new(entry.base as *mut u8).unwrap(),
                    NonNull::new((entry.base + entry.len) as *mut u8).unwrap(),
                );
            }
        }
    }

    dbg!("{}", buddy.0.stats);
}
