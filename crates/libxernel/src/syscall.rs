// constants for syscall numbers

use bitflags::bitflags;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_MMAP: usize = 4;

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
    InvalidArgument = -11,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct ProtectionFlags: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct MapFlags: u8 {
        const SHARED = 1 << 0;
        const PRIVATE = 1 << 1;
        const ANONYMOUS = 1 << 3;
    }
}
