use alloc::vec::Vec;
use libxernel::syscall::{MapFlags, ProtectionFlags};
use x86_64::align_up;
use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    VirtAddr,
};

use super::MMAP_START_ADDR;

pub struct VmEntry {
    start: VirtAddr,
    end: VirtAddr, // TODO: remove end
    length: usize,
    prot: ProtectionFlags,
    flags: MapFlags,
    // TODO: add something to represent to which file this entry belongs to
    file: Option<()>,
}

pub struct Vm {
    entries: Vec<VmEntry>,
}

impl Vm {
    pub const fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, start: VirtAddr, length: usize, prot: ProtectionFlags, flags: MapFlags) {
        let end = start + length;
        self.entries.push(VmEntry {
            start,
            end,
            length,
            prot,
            flags,
            file: None,
        });
    }

    pub fn find_next_start_address(&self) -> VirtAddr {
        let last_entry = self.entries.last();

        if let Some(last_entry) = last_entry {
            VirtAddr::new(align_up(last_entry.end.as_u64(), Size4KiB::SIZE))
        } else {
            VirtAddr::new(MMAP_START_ADDR as u64)
        }
    }

    pub fn clean_up(&mut self) {
        todo!("clean up all mappings and free memory")
        // NOTE: don't forget to remove the entries from the vector
    }
}
