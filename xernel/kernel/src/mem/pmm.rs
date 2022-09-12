use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineMmapRequest};
use x86_64::{PhysAddr, structures::paging::PhysFrame};

pub const FRAME_SIZE: u64 = 4096;

static mut USABLE_FRAME_COUNT: u64 = 0;
static MMAP_REQUEST: LimineMmapRequest = LimineMmapRequest::new(0);

lazy_static! {
    static ref FRAME_LIST: FrameList = FrameList {
        mmap: MMAP_REQUEST
            .get_response()
            .get()
            .expect("barebones: recieved no mmap")
            .mmap()
            .unwrap()
    };
}

struct FrameList {
    mmap: &'static [LimineMemmapEntry],
}

impl FrameList {
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
}

pub fn init() {
    for entry in FRAME_LIST.mmap {
        if entry.typ == LimineMemoryMapEntryType::Usable {
            unsafe {
                USABLE_FRAME_COUNT += entry.len / FRAME_SIZE as u64;
            }
        }
    }

    // create the bitmap
    let bitmap_size = FRAME_LIST.len() / 8;
    let mut bitmap_frame_count = (bitmap_size / FRAME_SIZE) as usize;

    if bitmap_size % FRAME_SIZE as u64 != 0 {
        bitmap_frame_count += 1;
    }

    for i in 0..bitmap_frame_count {
        set_bit(i);
    }
}

fn set_bit(index: usize) {
    let frame_index = index / 8 / FRAME_SIZE as usize;
    let bit_index = index % (FRAME_SIZE as usize * 8);

    let frame_addr = FRAME_LIST.index(frame_index);
    let byte_offset = bit_index / 8;
    let bit_offset = bit_index % 8;

    let byte_addr = (frame_addr.start_address().as_u64() + byte_offset as u64) as *mut u8;

    unsafe {
        let byte = byte_addr.read_volatile();
        byte_addr.write_volatile(byte | (1 << bit_offset));
    }
}

fn get_bit(index: usize) -> bool {
    let frame_index = index / 8 / FRAME_SIZE as usize;
    let bit_index = index % (FRAME_SIZE as usize * 8);

    let frame_addr = FRAME_LIST.index(frame_index);
    let byte_offset = bit_index / 8;
    let bit_offset = bit_index % 8;

    let byte_addr = (frame_addr.start_address().as_u64() + byte_offset as u64) as *mut u8;

    unsafe {
        let byte = byte_addr.read_volatile();
        byte & (1 << bit_offset) != 0
    }
}

fn clear_bit(index: usize) {
    let frame_index = index / 8 / FRAME_SIZE as usize;
    let bit_index = index % (FRAME_SIZE as usize * 8);

    let frame_addr = FRAME_LIST.index(frame_index);
    let byte_offset = bit_index / 8;
    let bit_offset = bit_index % 8;

    let byte_addr = (frame_addr.start_address().as_u64() + byte_offset as u64) as *mut u8;

    unsafe {
        let byte = byte_addr.read_volatile();
        byte_addr.write_volatile(byte & !(1 << bit_offset));
    }
}

static mut LAST_ALLOCATED_FRAME_INDEX: u64 = 0;

unsafe fn search_bit() -> Option<usize> {
    // TODO: optimize by checking multiple bits at once

    let mut current_index: u64 = LAST_ALLOCATED_FRAME_INDEX + 1;

    loop {
        if current_index >= FRAME_LIST.len() {
            current_index = 0;
        }

        if current_index == LAST_ALLOCATED_FRAME_INDEX {
            return None;
        }

        let is_used = get_bit(current_index as usize);

        if !is_used {
            LAST_ALLOCATED_FRAME_INDEX = current_index;
            set_bit(current_index as usize);

            return Some(current_index as usize);
        }

        current_index += 1;
    }
}

pub fn alloc() -> Option<PhysFrame> {
    let idx = unsafe { search_bit()? };

    Some(FRAME_LIST.index(idx))
}

pub fn free(addr: PhysAddr) {
    let idx = FRAME_LIST.addr_to_index(addr);
    clear_bit(idx);
}

pub fn free_frame(addr: PhysFrame) {
    free(addr.start_address());
}
