use super::{Result, pathbuf::PathBuf, vnode::VNode};
use alloc::{string::String, sync::Arc};
use libxernel::sync::Spinlock;

// According to BSD each Mount object has a pointer to vfsops and to private data
// As in vnode we combine the member which holds the vfs operations and the private data which is used by the file system
pub struct Mount {
    /// Operations vector including private data for file system
    mnt_op_data: Arc<Spinlock<dyn VfsOps>>,
    /// VNode we are mounted on
    /// None if root node
    vnode_covered: Option<Arc<Spinlock<VNode>>>,
    flags: u64,
}

impl Mount {
    pub fn new(driver: Arc<Spinlock<dyn VfsOps>>, vnode_covered: Option<Arc<Spinlock<VNode>>>) -> Self {
        Mount {
            mnt_op_data: driver,
            vnode_covered,
            flags: 0,
        }
    }
}

impl Mount {
    pub fn vfs_mount(&mut self, path: String) {
        self.mnt_op_data.lock().vfs_mount(path)
    }

    pub fn vfs_start(&mut self) {
        self.mnt_op_data.lock().vfs_start()
    }

    pub fn vfs_unmount(&self) {
        self.mnt_op_data.lock().vfs_unmount()
    }

    pub fn vfs_root(&self) -> Result<Arc<Spinlock<VNode>>> {
        self.mnt_op_data.lock().vfs_root()
    }

    pub fn vfs_statvfs(&self) {
        self.mnt_op_data.lock().vfs_statvfs()
    }

    pub fn vfs_sync(&self) {
        self.mnt_op_data.lock().vfs_sync()
    }

    pub fn vfs_vget(&self) {
        self.mnt_op_data.lock().vfs_vget()
    }

    pub fn vfs_lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>> {
        self.mnt_op_data.lock().vfs_lookup(path)
    }

    pub fn vfs_init(&mut self) {
        self.mnt_op_data.lock().vfs_init()
    }

    pub fn vfs_done(&self) {
        self.mnt_op_data.lock().vfs_done()
    }

    pub fn vfs_name(&self) -> String {
        self.mnt_op_data.lock().vfs_name()
    }
}

/// Operations supported on mounted file system
/// Has an extra method called `name` since Rust traits don't support variables, with trait objects, the `name` method returns the vfs_name
pub trait VfsOps {
    /// Mounts a new instance of the file system.
    fn vfs_mount(&mut self, path: String);

    /// Makes the file system operational.
    fn vfs_start(&mut self);

    /// Unmounts an instance of the file system.
    fn vfs_unmount(&self);

    /// Gets the file system root vnode.
    fn vfs_root(&self) -> Result<Arc<Spinlock<VNode>>>;

    /// Gets file system statistics.
    fn vfs_statvfs(&self) {
        todo!("{} does not implement vfs_statvfs", self.vfs_name());
    }

    /// Flushes file system buffers.
    fn vfs_sync(&self);

    /// Gets a vnode from a file identifier.
    fn vfs_vget(&self);

    fn vfs_lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>>;

    /// Initializes the file system driver.
    fn vfs_init(&mut self);

    /// Reinitializes the file system driver.
    fn vfs_reinit(&self) {
        todo!("{} does not implement vfs_reinit", self.vfs_name());
    }

    /// Finalizes the file system driver.
    fn vfs_done(&self);

    /// Mounts an instance of the file system as the root file system.
    fn vfs_mountroot(&self) {
        todo!("{} does not implement vfs_mountroot", self.vfs_name());
    }

    /// Returns the name of the file system
    fn vfs_name(&self) -> String;
}
