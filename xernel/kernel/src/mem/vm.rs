use alloc::vec::Vec;
use libxernel::syscall::{MapFlags, ProtectionFlags};
use x86_64::VirtAddr;

pub struct VmEntry {
    start: VirtAddr,
    // TODO: should we remove one of these as it is reduntant?
    end: VirtAddr,
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

    pub fn clean_up(&mut self) {
        todo!("clean up all mappings and free memory")
        // NOTE: don't forget to remove the entries from the vector
    }
}
