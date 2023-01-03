///! The design and implementaiton of this virtual file system is influenced by BSD (NetBSD in particular).
///! BSD was chosen over Linux since the architecture of the VFS made more sense in regards of naming and so on.
///! NetBSD was my choice since it had the simplest codebase to read through.
mod error;
mod mount;
pub mod tmpfs;
pub mod vfs;
pub mod vnode;
