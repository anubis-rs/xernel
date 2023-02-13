use alloc::vec::Vec;
use x86_64::VirtAddr;

pub struct VmEntry {
    start: VirtAddr,
    end: VirtAddr,
    length: usize,
}

pub struct Vm {
    entries: Vec<VmEntry>,
}

impl Vm {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}
