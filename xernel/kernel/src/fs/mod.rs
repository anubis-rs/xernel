pub mod error;
pub mod mountpoint;
pub mod ramfs;
pub mod vfs;

use error::Result;
use libxernel::sync::Spinlock;

use core::ptr::NonNull;

use alloc::{
    rc::Rc,
    string::{String, ToString},
};

pub use vfs::Vfs;

pub static VFS: Spinlock<Vfs> = Spinlock::new(Vfs::new());

// TODO: Return result instead of plain value
pub trait FsNodeHandler {
    fn read(&self, buf: &mut [u8], count: usize, offset: usize) -> Result<usize>;
    fn write(&self, buf: &mut [u8], count: usize, offset: usize) -> usize;
    fn open(&self);
    fn close(&self);
    fn readdir(&self);
    fn finddir(&self);
    fn create(&self);
    fn mkdir(&self);
}

pub struct FsNode {
    name: String,
    uid: u32,
    gid: u32,
    flags: u32,
    inode: u32,
    offset: u32,
    length: u32,
    handler: Rc<dyn FsNodeHandler>,
}

impl FsNode {
    pub fn new(name: String, handler: Rc<dyn FsNodeHandler>) -> Self {
        FsNode {
            name: name,
            uid: 0,
            gid: 0,
            flags: 0,
            inode: 0,
            offset: 0,
            length: 0,
            handler: handler,
        }
    }
}
