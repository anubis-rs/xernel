use alloc::collections::BTreeMap;
use libxernel::syscall::{MapFlags, ProtectionFlags};
use x86_64::structures::paging::{PageTableFlags, PhysFrame};
use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    VirtAddr,
};

use crate::cpu::current_process;
use crate::mem::PROCESS_END;

use super::frame::FRAME_ALLOCATOR;
use super::{PROCESS_START, STACK_SIZE};

pub struct VmEntry {
    pub start: VirtAddr,
    pub length: usize,
    pub prot: ProtectionFlags,
    pub flags: MapFlags,
    // TODO: add something to represent to which file this entry belongs to
    file: Option<()>,
}

impl VmEntry {
    pub fn end(&self) -> VirtAddr {
        self.start + self.length as u64
    }

    pub fn unmap(&self) {
        let process = current_process();
        let mut process = process.lock();

        // SAFETY: only userspace processes should have Vm mappings
        let page_mapper = process.get_page_table().as_mut().unwrap();
        let mut frame_allocator = FRAME_ALLOCATOR.lock();

        for page in (self.start..self.end()).step_by(Size4KiB::SIZE as usize) {
            if let Some(phys_addr) = page_mapper.translate(page) {
                unsafe {
                    frame_allocator.deallocate_frame(PhysFrame::<Size4KiB>::containing_address(phys_addr));
                }
            }

            page_mapper.unmap(page);
        }
    }
}

pub struct Vm {
    entries: BTreeMap<VirtAddr, VmEntry>,
}

impl Vm {
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    fn add_entry(&mut self, start: VirtAddr, length: usize, prot: ProtectionFlags, flags: MapFlags) {
        let entry = VmEntry {
            start,
            length,
            prot,
            flags,
            file: None,
        };

        self.entries.insert(start, entry);
    }

    pub fn is_available(&self, start: VirtAddr, length: usize) -> bool {
        let start = start.as_u64();

        !self.entries.iter().any(|(_, entry)| {
            entry.start.as_u64() < start && entry.end().as_u64() + Size4KiB::SIZE > start
                || start + length as u64 + Size4KiB::SIZE > entry.start.as_u64()
                    && (start + length as u64 + Size4KiB::SIZE) < entry.end().as_u64() + Size4KiB::SIZE
        })
    }

    pub fn create_entry_low(&mut self, length: usize, prot: ProtectionFlags, flags: MapFlags) -> VirtAddr {
        self.create_entry_at(VirtAddr::new(PROCESS_START), length, prot, flags)
    }

    pub fn create_entry_high(&mut self, length: usize, prot: ProtectionFlags, flags: MapFlags) -> VirtAddr {
        let mut start_address = VirtAddr::new(PROCESS_END - length as u64);

        loop {
            if self.is_available(start_address, length) {
                if start_address.as_u64() < PROCESS_START {
                    panic!(
                        "create_entry_high: {:x}(length = {}) is out of bounds",
                        start_address, length
                    );
                }

                self.add_entry(start_address, length, prot, flags);
                return start_address;
            }

            // NOTE: at the moment only a stack should be create at the high end of the process address space
            start_address -= STACK_SIZE;
        }
    }

    /// A new entry is created at the given address or higher
    pub fn create_entry_at(
        &mut self,
        mut start: VirtAddr,
        length: usize,
        prot: ProtectionFlags,
        flags: MapFlags,
    ) -> VirtAddr {
        if start.as_u64() + length as u64 > PROCESS_END {
            panic!("create_entry_at: {:x}(length = {}) is out of bounds", start, length);
        }

        if !start.is_aligned(Size4KiB::SIZE) {
            panic!("create_entry_at: {:x} is not aligned", start);
        }

        if start.as_u64() < PROCESS_START {
            start = VirtAddr::new(PROCESS_START);
        }

        if self.is_available(start, length) {
            self.add_entry(start, length, prot, flags);
            return start;
        }

        let mut values_iter = self.entries.values();
        let mut previous = values_iter.next().unwrap();
        let current = values_iter.next();

        if current.is_none() {
            let new_start = previous.end() + Size4KiB::SIZE;
            let new_start = new_start.align_up(Size4KiB::SIZE);

            self.add_entry(new_start, length, prot, flags);
            return new_start;
        }

        let mut current = current.unwrap();

        loop {
            if current.start - previous.end() >= length as u64 + 2 * Size4KiB::SIZE {
                let new_start = previous.end() + Size4KiB::SIZE;
                let new_start = new_start.align_up(Size4KiB::SIZE);

                self.add_entry(new_start, length, prot, flags);
                return new_start;
            }

            previous = current;
            let current_opt = values_iter.next();

            if current_opt.is_none() {
                let new_start = previous.end() + Size4KiB::SIZE;
                let new_start = new_start.align_up(Size4KiB::SIZE);

                if new_start.as_u64() + length as u64 > PROCESS_END {
                    panic!(
                        "create_entry_at: {:x}(length = {}) is out of bounds! Vm space is exhausted",
                        new_start, length
                    );
                }

                self.add_entry(new_start, length, prot, flags);
                return new_start;
            }

            current = current_opt.unwrap();
        }
    }

    pub fn get_entry_from_address(&self, addr: VirtAddr) -> Option<&VmEntry> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.start <= addr && entry.end() > addr)
            .map(|(_, entry)| entry)
    }

    pub fn clean_up(&mut self) {
        self.entries.values().for_each(|value| value.unmap());
        self.entries.clear();
    }
}

pub fn ptflags_from_protflags(flags: ProtectionFlags, user_accessible: bool) -> PageTableFlags {
    let mut new_flags = PageTableFlags::PRESENT;

    if user_accessible {
        new_flags |= PageTableFlags::USER_ACCESSIBLE;
    }

    if !flags.contains(ProtectionFlags::READ) {
        // NOTE: it is not possible to remove read access from a page
    }

    if flags.contains(ProtectionFlags::WRITE) {
        new_flags |= PageTableFlags::WRITABLE;
    }

    if !flags.contains(ProtectionFlags::EXECUTE) {
        new_flags |= PageTableFlags::NO_EXECUTE;
    }

    new_flags
}

pub fn protflags_from_ptflags(flags: PageTableFlags) -> ProtectionFlags {
    let mut new_flags = ProtectionFlags::empty();

    if flags.contains(PageTableFlags::WRITABLE) {
        new_flags |= ProtectionFlags::WRITE;
    }

    if !flags.contains(PageTableFlags::NO_EXECUTE) {
        new_flags |= ProtectionFlags::EXECUTE;
    }

    new_flags
}
