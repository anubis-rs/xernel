//! The design and implementation of this virtual file system is heavily influenced by BSD.

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
