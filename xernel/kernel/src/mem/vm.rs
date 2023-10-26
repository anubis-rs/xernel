use alloc::vec::Vec;
use bitflags::bitflags;
use x86_64::VirtAddr;

bitflags! {
    pub struct ProtFlags: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

pub struct VmEntry {
    start: VirtAddr,
    // TODO: should we remove one of these as it is reduntant?
    end: VirtAddr,
    length: usize,
    prot: ProtFlags,
}

pub struct Vm {
    entries: Vec<VmEntry>,
}

impl Vm {
    pub const fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, start: VirtAddr, length: usize, prot: ProtFlags) {
        let end = start + length;
        self.entries.push(VmEntry {
            start,
            end,
            length,
            prot,
        });
    }

    pub fn clean_up(&mut self) {
        todo!("clean up all mappings and free memory")
        // NOTE: don't forget to remove the entries from the vector
    }
}
