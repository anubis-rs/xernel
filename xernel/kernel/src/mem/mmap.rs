use libxernel::syscall::{MapFlags, ProtectionFlags, SyscallError};
use x86_64::{
    structures::{
        idt::PageFaultErrorCode,
        paging::{Page, PageSize, Size4KiB},
    },
    VirtAddr,
};

use crate::{allocator::align_up, sched::scheduler::Scheduler};

use super::{frame::FRAME_ALLOCATOR, vm::ptflags_from_protflags};

#[allow(unused_variables)]
pub fn mmap(
    addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> Result<isize, SyscallError> {
    let addr = VirtAddr::new(addr as u64);
    let prot = ProtectionFlags::from_bits(prot as u8).ok_or(SyscallError::InvalidArgument)?;
    let flags = MapFlags::from_bits(flags as u8).ok_or(SyscallError::InvalidArgument)?;
    let len = align_up(len, Size4KiB::SIZE as usize);

    let process = Scheduler::current_process();
    let mut process = process.lock();

    match flags {
        MapFlags::ANONYMOUS => {
            let start_address = process.vm().create_entry_at(addr, len, prot, flags);

            Ok(start_address.as_u64() as isize)
        }
        _ => todo!("mmap: implement MAP_SHARED and MAP_PRIVATE"),
    }
}

/// Handles a page fault and returns whether the fault was handled successfully
pub fn handle_page_fault(addr: VirtAddr, error_code: PageFaultErrorCode) -> bool {
    let process = Scheduler::current_process();
    let mut process = process.lock();

    let vm_entry = process.vm().get_entry_from_address(addr);

    if let Some(vm_entry) = vm_entry {
        if vm_entry.flags != MapFlags::ANONYMOUS {
            todo!("handle_page_fault: implement non-anonymous mappings");
        }

        // If the page is present we don't need to map it
        // FIXME: this doesn't work when COW is implemented
        if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            return false;
        }

        let base_addr = addr.align_down(Size4KiB::SIZE);
        let frame = FRAME_ALLOCATOR.lock().allocate_frame::<Size4KiB>().unwrap();

        let pt_flags = ptflags_from_protflags(vm_entry.prot, process.page_table.is_some());
        let mut pt = process.get_page_table().unwrap();

        pt.map::<Size4KiB>(frame, Page::from_start_address(base_addr).unwrap(), pt_flags, true);

        true
    } else {
        false
    }
}
