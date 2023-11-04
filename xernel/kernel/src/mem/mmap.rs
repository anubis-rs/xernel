use libxernel::syscall::{MapFlags, ProtectionFlags, SyscallError};
use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    VirtAddr,
};

use crate::{allocator::align_up, sched::scheduler::Scheduler};

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
            let start_address = process.vm().find_next_start_address();
            process.vm().add_entry(start_address, len, prot, flags);

            Ok(start_address.as_u64() as isize)
        }
        _ => todo!("mmap: implement MAP_SHARED and MAP_PRIVATE"),
    }
}
