// constants for syscall numbers

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(isize)]
pub enum SyscallError {
    NoPermission = -1,
}
