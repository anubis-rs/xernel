// constants for syscall numbers

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(isize)]
pub enum SyscallError {
    NoPermission = -1,
    VNodeNotFound = -2,
    NotADirectory = -3,
    IsADirectory = -4,
    NoSpace = -5,
    NotEmpty = -6,
    EntryNotFound = -7,
    MountPointNotFound = -8,
    FileSystemNotFound = -9,
    MalformedPath = -10,
}
