use libxernel::syscall::{MapFlags, ProtectionFlags, SyscallError};
use x86_64::VirtAddr;

use crate::sched::scheduler::Scheduler;

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

    let process = Scheduler::current_process();

    todo!("mmap")
}
