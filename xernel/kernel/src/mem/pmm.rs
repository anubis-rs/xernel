use limine::{LimineMemmapEntry, LimineMmapRequest, LimineMemoryMapEntryType};

use crate::{println, print};

static mut USABLE_PAGE_COUNT: u64 = 0;

static MMAP_Request: LimineMmapRequest = LimineMmapRequest::new(0);

lazy_static! {
    static ref PAGE_LIST: PageList = PageList {
        mmap: MMAP_Request.get_response()
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
    fn new(mmap: &'static [LimineMemmapEntry]) -> Self {
        Self { mmap }
    }

    fn len(&self) -> u64 {
        unsafe { USABLE_PAGE_COUNT }
    }
}

struct Page {
    addr: usize,
    isUsed: bool,
}

pub fn init() {
    for entry in PAGE_LIST.mmap {
        if entry.typ == LimineMemoryMapEntryType::Usable {
            unsafe {
                USABLE_PAGE_COUNT += entry.len / 4096;
            }
        }
    }

    println!("usable page count: {}", PAGE_LIST.len());
}
