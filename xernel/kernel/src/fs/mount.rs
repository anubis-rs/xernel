use super::vnode::VNode;
use alloc::{string::String, sync::Arc, vec::Vec};
use libxernel::sync::Spinlock;

// According to BSD each Mount object has a pointer to vfsops and to private data
// As in vnode we combine the member which holds the vfs operations and the private data which is used by the file system
// FIXME: Fix cyclic Arc's
pub struct Mount {
    /// Operations vector including private data for file system
    pub mnt_op_data: Arc<Spinlock<dyn VfsOps>>,
    /// VNode we are mounted on
    /// None if root node
    vnode_covered: Option<Arc<Spinlock<VNode>>>,
    vnode_list: Vec<Arc<Spinlock<VNode>>>,
    flags: u64,
}

impl Mount {
    pub fn new(
        driver: Arc<Spinlock<dyn VfsOps>>,
        vnode_covered: Option<Arc<Spinlock<VNode>>>,
    ) -> Self {
        Mount {
            mnt_op_data: driver,
            vnode_covered: vnode_covered,
            vnode_list: Vec::new(),
            flags: 0,
        }
    }
}

/// Operations supported on mounted file system
/// Has an extra method called `name` since Rust traits don't support variables, with trait objects, the `name` method returns the vfs_name
pub trait VfsOps {
    /// Mounts a new instance of the file system.
    fn vfs_mount(&mut self, path: String);

    /// Makes the file system operational.
    fn vfs_start(&self);

    /// Unmounts an instance of the file system.
    fn vfs_unmount(&self);

    /// Gets the file system root vnode.
    fn vfs_root(&self);

    /// Queries or modifies space quotas.
    fn vfs_quotactl(&self);

    /// Gets file system statistics.
    fn vfs_statvfs(&self);

    /// Flushes file system buffers.
    fn vfs_sync(&self);

    /// Gets a vnode from a file identifier.
    fn vfs_vget(&self);

    fn vfs_lookup(&self, path: String) -> Arc<Spinlock<VNode>>;

    /// Converts a NFS file handle to a vnode.
    fn vfs_fhtovp(&self);

    /// Converts a vnode to a NFS file handle.
    fn vfs_vptofh(&self);

    /// Initializes the file system driver.
    fn vfs_init(&mut self);

    /// Reinitializes the file system driver.
    fn vfs_reinit(&self) {
        unimplemented!("{} does not implement fs_reinit", self.vfs_name());
    }

    /// Finalizes the file system driver.
    fn vfs_done(&self);

    /// Mounts an instance of the file system as the root file system.
    fn vfs_mountroot(&self) {
        unimplemented!("{} does not implement fs_mountroot", self.vfs_name());
    }

    /// Controls extended attributes.
    // The generic vfs_stdextattrctl function is provided as a simple hook for file system that do not support this operation
    // TODO: create a generic vfs_stdextattrctl function
    fn vfs_extattrctl(&self);

    /// Returns the name of the file system
    fn vfs_name(&self) -> &str;
}
