/// The implementation of this virtual file system is influenced by BSD (NetBSD in particular).
/// BSD was chosen over Linux since the architecture of the VFS made more sense in regards of naming and so on.
/// NetBSD was our choice because it had the simplest codebase to read through.
mod error;
mod mount;
pub mod vfs;
pub mod vnode;
