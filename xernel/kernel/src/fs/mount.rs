use super::vnode::VNode;
use alloc::{sync::Arc, vec::Vec};

// According to BSD each Mount object has a pointer to vfsops and to private data
// As in vnode we combine the member which holds the vfs operations and the private data which is used by the file system
pub struct Mount {
    mnt_op_data: Arc<dyn VfsOps>,
    vnode_covered: Arc<VNode>,
    root_node: Arc<VNode>,
    vnode_list: Vec<Arc<VNode>>,
    flags: u64,
}

/// Operatiosn supported on mounted file system
/// Has an extra method called `name` since Rust traits don't support variables, with trait objects, the `name` method returns the vfs_name
trait VfsOps {
    /// Mounts a new instance of the file system.
    fn fs_mount(&self);

    /// Makes the file system operational.
    fn fs_start(&self);

    /// Unmounts an instance of the file system.
    fn fs_unmount(&self);

    /// Gets the file system root vnode.
    fn fs_root(&self);

    /// Queries or modifies space quotas.
    fn fs_quotactl(&self);

    /// Gets file system statistics.
    fn fs_statvfs(&self);

    /// Flushes file system buffers.
    fn fs_sync(&self);

    /// Gets a vnode from a file identifier.
    fn fs_vget(&self);

    /// Converts a NFS file handle to a vnode.
    fn fs_fhtovp(&self);

    /// Converts a NFS file handle to a vnode.
    fn fs_vptofh(&self);

    /// Initializes the file system driver.
    fn fs_init(&self);

    /// Reinitializes the file system driver.
    fn fs_reinit(&self) {
        unimplemented!("{} does not implement fs_reinit", self.fs_name());
    }

    /// Finalizes the file system driver.
    fn fs_done(&self);

    /// Mounts an instance of the file system as the root file system.
    fn fs_mountroot(&self) {
        unimplemented!("{} does not implement fs_mountroot", self.fs_name());
    }

    /// Controls extended attributes.
    // The generic vfs_stdextattrctl function is provided as a simple hook for file system that do not support this operation
    // TODO: create a generic vfs_stdextattrctl function
    fn fs_extattrctl(&self);

    /// Returns the name of the file system
    fn fs_name(&self) -> &str;
}
