use core::ptr::NonNull;

use crate::{allocator::buddy::BuddyAllocator, mem::HIGHER_HALF_OFFSET};
use libxernel::addr::PhysAddr;
use libxernel::paging::{PageSize, PhysFrame};
use libxernel::sync::{Once, Spinlock};
use limine::{MemmapEntry, MemmapRequest, MemoryMapEntryType, NonNullPtr};

static MMAP_REQUEST: MemmapRequest = MemmapRequest::new(0);

pub static MEMORY_MAP: Once<&'static [NonNullPtr<MemmapEntry>]> = Once::new();

pub struct PhysFrameAllocator(BuddyAllocator<{ super::FRAME_SIZE as usize }, 12>); // maximum allocation size is 16mb

pub static FRAME_ALLOCATOR: Spinlock<PhysFrameAllocator> = Spinlock::new(PhysFrameAllocator(BuddyAllocator::new()));

impl PhysFrameAllocator {
    pub fn allocate_frame<P: PageSize>(&mut self) -> Option<PhysFrame<P>> {
        let order = self.0.order_for_size(P::SIZE as usize);

        let frame = self.0.allocate(order);
        let start_addr = frame.unwrap().as_ptr() as u64 - *HIGHER_HALF_OFFSET;
        let pframe = PhysFrame::from_start_address(PhysAddr::new(start_addr));
        pframe.ok()
    }

    pub unsafe fn deallocate_frame<P: PageSize>(&mut self, frame: PhysFrame<P>) {
        let order = self.0.order_for_size(P::SIZE as usize);

        self.0
            .deallocate(
                NonNull::new((frame.start_address().as_u64() + *HIGHER_HALF_OFFSET) as *mut u8).unwrap(),
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
        if entry.typ == MemoryMapEntryType::Usable {
            unsafe {
                buddy
                    .0
                    .add_region(
                        NonNull::new((entry.base + *HIGHER_HALF_OFFSET) as *mut u8).unwrap(),
                        NonNull::new((entry.base + *HIGHER_HALF_OFFSET + entry.len) as *mut u8).unwrap(),
                    )
                    .unwrap();
            }
        }
    }

    dbg!("{}", buddy.0.stats);
}
