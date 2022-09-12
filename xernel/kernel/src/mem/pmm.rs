use limine::{LimineMemmapEntry, LimineMemoryMapEntryType, LimineMmapRequest};
use x86_64::PhysAddr;

use crate::{print, println};

pub const PAGE_SIZE: u64 = 4096;

static mut USABLE_PAGE_COUNT: u64 = 0;
static MMAP_REQUEST: LimineMmapRequest = LimineMmapRequest::new(0);

lazy_static! {
    static ref PAGE_LIST: PageList = PageList {
        mmap: MMAP_REQUEST
            .get_response()
            .get()
            .expect("barebones: recieved no mmap")
            .mmap()
            .unwrap()
    };
}

struct PageList {
    mmap: &'static [LimineMemmapEntry],
}

impl PageList {
    fn len(&self) -> u64 {
        unsafe { USABLE_PAGE_COUNT }
    }

    fn index(&self, index: usize) -> PhysAddr {
        if index >= self.len() as usize {
            panic!("PageList index out of bounds");
        }

        let mut page_count = 0;

        for entry in self.mmap {
            if entry.typ == LimineMemoryMapEntryType::Usable {
                let entry_pages = entry.len / PAGE_SIZE as u64;

                // check if the index is in this entry
                if index < page_count + entry_pages as usize {
                    // calculate the offset
                    let diff = (index - page_count) as u64;

                    // calculate the address
                    let addr = entry.base + diff * PAGE_SIZE;

                    return PhysAddr::new(addr);
                }

                page_count += entry_pages as usize;
            }
        }

        unreachable!();
    }

    fn addr_to_index(&self, addr: PhysAddr) -> usize {
        let mut page_count = 0;

        for entry in self.mmap {
            if entry.typ == LimineMemoryMapEntryType::Usable {
                let entry_pages = entry.len / PAGE_SIZE as u64;

                // check if address is in this entry
                if addr.as_u64() >= entry.base && addr.as_u64() < entry.base + entry.len {
                    // calculate the offset
                    let diff = addr.as_u64() - entry.base;

                    // calculate the index
                    let index = page_count + (diff / PAGE_SIZE) as usize;

                    return index;
                }

                page_count += entry_pages as usize;
            }
        }

        panic!("PageList addr_to_index out of bounds");
    }
}

pub fn init() {
    for entry in PAGE_LIST.mmap {
        if entry.typ == LimineMemoryMapEntryType::Usable {
            unsafe {
                USABLE_PAGE_COUNT += entry.len / PAGE_SIZE as u64;
            }
        }
    }

    // create the bitmap
    let bitmap_size = PAGE_LIST.len() / 8;
    let mut bitmap_page_count = (bitmap_size / PAGE_SIZE) as usize;

    if bitmap_size % PAGE_SIZE as u64 != 0 {
        bitmap_page_count += 1;
    }

    println!("bitmap page count: {}", bitmap_page_count);
    for i in 0..bitmap_page_count {
        set_bit(i);
    }
}

fn set_bit(index: usize) {
    let page_index = index / 8 / PAGE_SIZE as usize;
    let bit_index = index % (PAGE_SIZE as usize * 8);

    let page_addr = PAGE_LIST.index(page_index);
    let byte_offset = bit_index / 8;
    let bit_offset = bit_index % 8;

    let byte_addr = (page_addr.as_u64() + byte_offset as u64) as *mut u8;

    unsafe {
        let byte = byte_addr.read_volatile();
        byte_addr.write_volatile(byte | (1 << bit_offset));
    }
}

fn clear_bit(index: usize) {
    let page_index = index / 8 / PAGE_SIZE as usize;
    let bit_index = index % (PAGE_SIZE as usize * 8);

    let page_addr = PAGE_LIST.index(page_index);
    let byte_offset = bit_index / 8;
    let bit_offset = bit_index % 8;

    let byte_addr = (page_addr.as_u64() + byte_offset as u64) as *mut u8;

    unsafe {
        let byte = byte_addr.read_volatile();
        byte_addr.write_volatile(byte & !(1 << bit_offset));
    }
}

fn search_bit() -> Option<usize> {
    // TODO: optimize by remembering the last index

    let mut current_page: u64 = 0;

    loop {
        let addr = PAGE_LIST.index(current_page as usize);

        // check page
        for i in 0..PAGE_SIZE {
            let byte_addr = (addr.as_u64() + i) as *mut u8;

            // check if it is ok to read this byte
            if (current_page * PAGE_SIZE * 8) as usize + (i * 8) as usize
                >= PAGE_LIST.len() as usize
            {
                return None;
            }

            unsafe {
                let byte = byte_addr.read_volatile();

                if byte != 0xFF {
                    // find the bit
                    for j in 0..8 {
                        let is_set = byte & (1 << j) != 0;
                        let idx = (current_page * PAGE_SIZE * 8) as usize + (i * 8 + j) as usize;

                        if idx > PAGE_LIST.len() as usize {
                            return None;
                        }

                        byte_addr.write_volatile(byte | (1 << j));

                        if !is_set {
                            return Some(idx);
                        }
                    }
                }
            }
        }

        current_page += 1;
    }
}

pub fn alloc() -> Option<PhysAddr> {
    let idx = search_bit()?;

    Some(PAGE_LIST.index(idx))
}

pub fn free(addr: PhysAddr) {
    let idx = PAGE_LIST.addr_to_index(addr);
    clear_bit(idx);
}
