use super::mount::Mount;
use alloc::sync::Arc;

pub enum VType {
    Non,
    Reg,
    Dir,
    Blk,
    Chr,
    Lnk,
    Soxk,
    Fifo,
    Bad,
}

// Each Vnode gets a field file system specific handler which is a struct given by the file system driver which implements the VNode Operations trait
// since this struct can also be used for the file system to store file system specific data we combine the fields v_data and v_op of the mount struct from NetBSD.
pub struct VNode {
    /// ptr to vfs we are in
    /// filesystem to which the vnode (we are mounted to) belongs to
    vfsp: Arc<Mount>,
    /// Holds the vnode operations vector and the private data for fs in one member
    /// since the struct, which each fs, which implements the VNodeOperations trait can directly own the private fs data
    v_data_op: Arc<dyn VNodeOperations>,
    v_type: VType,
    flags: u64,
    // maybe like netbsd, use union https://github.com/NetBSD/src/blob/trunk/sys/sys/vnode.h#L172
    // used if vnode is mountpoint, v_mounted_here points to the other file system
    v_mounted_here: Arc<Mount>,
}

impl VNode {
    pub fn new(
        vfsp: Arc<Mount>,
        data_op: Arc<dyn VNodeOperations>,
        v_type: VType,
        v_mounted_here: Arc<Mount>,
    ) -> Self {
        VNode {
            vfsp: vfsp,
            v_data_op: data_op,
            v_type: v_type,
            v_mounted_here: v_mounted_here,
            flags: 0,
        }
    }
}

/// This trait maps logical operations to real functions. It is file system specific as the actions taken by each operation depend heavily on the file system where the file resides.
pub trait VNodeOperations {
    /// Aborts an in-progress operation.
    fn abortop(&self);

    /// Checks access permissions on a file.
    fn access(&self);

    fn advlock(&self);

    /// Maps a logical block number to a physical block number.
    fn bmap(&self);

    /// Writes a system buffer.
    fn bwrite(&self);

    /// Closes a file.
    fn close(&self);

    /// Creates a new file.
    fn create(&self);

    /// Synchronizes the file with on-disk contents.
    fn fsync(&self);

    /// Gets a file's attributes.
    fn getattr(&self);

    /// Marks the vnode as inactive.
    fn inactive(&self);

    /// Performs an ioctl on a file.
    fn ioctl(&self);

    /// Creates a new hard link for a file.
    fn link(&self);

    /// Performs a path name lookup.
    fn lookup(&self);

    /// Creates a new special file (a device or a named pipe).
    fn mknod(&self);

    /// Opens a file.
    fn open(&self);

    /// Returns pathconf information.
    fn pathconf(&self);

    /// Reads a chunk of data from a file.
    fn read(&self);

    /// Reads directory entries from a directory.
    fn readdir(&self);

    /// Reads the contents of a symbolic link.
    fn readlink(&self);

    /// Reclaims the vnode.
    fn reclaim(&self);

    /// Removes a file.
    fn remove(&self);

    /// Renames a file.
    fn rename(&self);

    /// Creates a new directory.
    fn mkdir(&self);

    /// Removes a directory.
    fn rmdir(&self);

    /// Sets a file's attributes.
    fn setattr(&self);

    /// Performs a file transfer between the file system's backing store and memory.
    fn strategy(&self);

    /// Creates a new symbolic link for a file.
    fn symlink(&self);

    /// Writes a chunk of data to a file.
    fn write(&self);

    fn kqfilter(&self);

    fn print(&self); // OpenBSD has it, NetBSD not?!

    /// Performs a fcntl on a file.
    fn fcntl(&self); // NetBSD has it, OpenBSD not?!
    /// Performs a poll on a file.
    fn poll(&self); // NetBSD has it, OpenBSD not?!
    /// Revoke access to a vnode and all aliases.
    fn revoke(&self); // NetBSD has it, OpenBSD not?!
    /// Maps a file on a memory region.
    fn mmap(&self); // NetBSD has it, OpenBSD not?!
    /// Test and inform file system of seek
    fn seek(&self); // NetBSD has it, OpenBSD not?!
    /// Truncates a file.
    fn truncate(&self); // NetBSD has it, OpenBSD not?!
    /// Updates a file's times.
    fn update(&self); // NetBSD has it, OpenBSD not?!
    /// Reads memory pages from the file.
    fn getpages(&self); // NetBSD has it, OpenBSD not?!
    /// Writes memory pages to the file.
    fn putpages(&self); // NetBSD has it, OpenBSD not?!
}
