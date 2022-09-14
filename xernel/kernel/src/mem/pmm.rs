use libxernel::spin::Spinlock;
use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineMmapRequest};
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    PhysAddr,
};

use super::HIGHER_HALF_OFFSET;

pub const FRAME_SIZE: u64 = 4096;

static mut USABLE_FRAME_COUNT: u64 = 0;
static MMAP_REQUEST: LimineMmapRequest = LimineMmapRequest::new(0);

// TODO: create struct for bit address (addres + offset) to remove duplicate code in get_bit, set_bit, clear_bit

lazy_static! {
    pub static ref MEMORY_MAP: &'static [LimineMemmapEntry] = MMAP_REQUEST
        .get_response()
        .get()
        .expect("barebones: recieved no mmap")
        .mmap()
        .unwrap();
}

pub static FRAME_ALLOCATOR: Spinlock<FrameAllocator> = Spinlock::new(FrameAllocator {
    mmap: &[],
    last_index: 0,
});

pub struct FrameAllocator {
    pub mmap: &'static [LimineMemmapEntry],
    last_index: u64,
}

impl FrameAllocator {
    fn len(&self) -> u64 {
        unsafe { USABLE_FRAME_COUNT }
    }

    fn index(&self, index: usize) -> PhysFrame {
        if index >= self.len() as usize {
            panic!("FrameList index out of bounds");
        }

        let mut frame_count = 0;

        for entry in self.mmap {
            if entry.typ == LimineMemoryMapEntryType::Usable {
                let entry_frames = entry.len / FRAME_SIZE as u64;

                // check if the index is in this entry
                if index < frame_count + entry_frames as usize {
                    // calculate the offset
                    let diff = (index - frame_count) as u64;

                    // calculate the address
                    let addr = entry.base + diff * FRAME_SIZE;

                    return PhysFrame::from_start_address(PhysAddr::new(addr)).unwrap();
                }

                frame_count += entry_frames as usize;
            }
        }

        unreachable!();
    }

    fn addr_to_index(&self, addr: PhysAddr) -> usize {
        let mut frame_count = 0;

        for entry in self.mmap {
            if entry.typ == LimineMemoryMapEntryType::Usable {
                let entry_pages = entry.len / FRAME_SIZE as u64;

                // check if address is in this entry
                if addr.as_u64() >= entry.base && addr.as_u64() < entry.base + entry.len {
                    // calculate the offset
                    let diff = addr.as_u64() - entry.base;

                    // calculate the index
                    let index = frame_count + (diff / FRAME_SIZE) as usize;

                    return index;
                }

                frame_count += entry_pages as usize;
            }
        }

        panic!("FrameList addr_to_index out of bounds");
    }

    fn set_bit(&self, index: usize) {
        let frame_index = index / 8 / FRAME_SIZE as usize;
        let bit_index = index % (FRAME_SIZE as usize * 8);

        let frame_addr = self.index(frame_index);
        let byte_offset = bit_index / 8;
        let bit_offset = bit_index % 8;

        let byte_addr = (*HIGHER_HALF_OFFSET
            + frame_addr.start_address().as_u64()
            + byte_offset as u64) as *mut u8;

        unsafe {
            let byte = byte_addr.read_volatile();
            byte_addr.write_volatile(byte | (1 << bit_offset));
        }
    }

    fn get_bit(&self, index: usize) -> bool {
        let frame_index = index / 8 / FRAME_SIZE as usize;
        let bit_index = index % (FRAME_SIZE as usize * 8);

        let frame_addr = self.index(frame_index);
        let byte_offset = bit_index / 8;
        let bit_offset = bit_index % 8;

        let byte_addr = (*HIGHER_HALF_OFFSET
            + frame_addr.start_address().as_u64()
            + byte_offset as u64) as *mut u8;

        unsafe {
            let byte = byte_addr.read_volatile();
            byte & (1 << bit_offset) != 0
        }
    }

    fn clear_bit(&self, index: usize) {
        let frame_index = index / 8 / FRAME_SIZE as usize;
        let bit_index = index % (FRAME_SIZE as usize * 8);

        let frame_addr = self.index(frame_index);
        let byte_offset = bit_index / 8;
        let bit_offset = bit_index % 8;

        let byte_addr = (*HIGHER_HALF_OFFSET
            + frame_addr.start_address().as_u64()
            + byte_offset as u64) as *mut u8;

        unsafe {
            let byte = byte_addr.read_volatile();
            byte_addr.write_volatile(byte & !(1 << bit_offset));
        }
    }

    unsafe fn search_bit(&mut self) -> Option<usize> {
        // TODO: optimize by checking multiple bits at once

        let mut current_index: u64 = self.last_index + 1;

        loop {
            if current_index >= self.len() {
                current_index = 0;
            }

            if current_index == self.last_index {
                return None;
            }

            let is_used = self.get_bit(current_index as usize);

            if !is_used {
                self.last_index = current_index;
                self.set_bit(current_index as usize);

                return Some(current_index as usize);
            }

            current_index += 1;
        }
    }
}

unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB> for FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let idx = unsafe { self.search_bit()? };

        Some(self.index(idx))
    }
}

impl x86_64::structures::paging::FrameDeallocator<Size4KiB> for FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let idx = self.addr_to_index(frame.start_address());
        self.clear_bit(idx);
    }
}

pub fn init() {
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    frame_allocator.mmap = &MEMORY_MAP;

    for entry in frame_allocator.mmap {
        if entry.typ == LimineMemoryMapEntryType::Usable {
            unsafe {
                USABLE_FRAME_COUNT += entry.len / FRAME_SIZE as u64;
            }
        }
    }

    // create the bitmap
    let bitmap_size = frame_allocator.len() / 8;
    let mut bitmap_frame_count = (bitmap_size / FRAME_SIZE) as usize;

    if bitmap_size % FRAME_SIZE as u64 != 0 {
        bitmap_frame_count += 1;
    }

    for i in 0..bitmap_frame_count {
        frame_allocator.set_bit(i);
    }
}
