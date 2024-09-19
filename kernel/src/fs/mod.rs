//! The design and implementation of this virtual file system is heavily influenced by BSD.

use core::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum VfsError {
    VNodeNotFound,
    NotADirectory,
    IsADirectory,
    NoSpace,
    NotEmpty,
    EntryNotFound,
    MountPointNotFound,
    FileSystemNotFound,
}

impl Display for VfsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        todo!()
    }
}

impl Error for VfsError {}

pub type Result<T, E = VfsError> = core::result::Result<T, E>;

pub mod file;
pub mod initramfs;
mod mount;
pub mod pathbuf;
pub mod tmpfs;
pub mod vfs;
pub mod vfs_syscalls;
pub mod vnode;

// todo: create fs init function which initializes vfs and initramfs

pub fn init() {}
