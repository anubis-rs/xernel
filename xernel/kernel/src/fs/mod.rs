//! The design and implementaiton of this virtual file system is heavily influenced by BSD.
//! BSD was chosen over Linux since the architecture of the VFS made more sense in regards of naming and so on.

#[derive(Debug)]
pub enum Error {
    VNodeNotFound,
    NotADirectory,
    IsADirectory,
    NoSpace,
    NotEmpty,
    EntryNotFound,
    MountPointNotFound,
    FileSystemNotFound,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub mod file;
mod mount;
pub mod pathbuf;
pub mod tmpfs;
pub mod vfs;
pub mod vfs_syscalls;
pub mod vnode;
