use libxernel::syscall::{MapFlags, ProtectionFlags, SyscallError};
use x86_64::VirtAddr;

use crate::sched::scheduler::Scheduler;

#[allow(unused_variables)]
pub fn mmap(
    addr: VirtAddr,
    len: usize,
    prot: ProtectionFlags,
    flags: MapFlags,
    fd: usize,
    offset: usize,
) -> Result<isize, SyscallError> {
    let process = Scheduler::current_process();

    todo!("mmap")
}
