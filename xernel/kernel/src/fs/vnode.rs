use super::error::Result;
use super::mount::Mount;
use super::pathbuf::PathBuf;
use alloc::string::String;
use alloc::{sync::Arc, sync::Weak};
use libxernel::sync::Spinlock;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum VType {
    Non,
    Regular,
    Directory,
    BlockDevice,
    CharacterDevice,
    SymbolicLink,
    Socket,
    Fifo,
    Bad,
}

// Each Vnode gets a field file system specific handler which is a struct given by the file system driver which implements the VNode Operations trait
// since this struct can also be used for the file system to store file system specific data we combine the fields v_data and v_op of the mount struct from NetBSD.
pub struct VNode {
    /// ptr to vfs we are in
    /// filesystem to which the vnode (we are mounted to) belongs to
    vfsp: Weak<Mount>,
    /// Holds the vnode operations vector and the private data for fs in one member
    /// since the struct, which each fs, which implements the VNodeOperations trait can directly own the private fs data
    v_data_op: Arc<Spinlock<dyn VNodeOperations>>,
    v_type: VType,
    flags: u64,
    // TODO: add attributes
    // maybe like netbsd, use union https://github.com/NetBSD/src/blob/trunk/sys/sys/vnode.h#L172
    // used if vnode is mountpoint, v_mounted_here points to the other file system
    v_mounted_here: Option<Weak<Mount>>,
}

impl VNode {
    pub fn new(
        vfsp: Weak<Mount>,
        data_op: Arc<Spinlock<dyn VNodeOperations>>,
        v_type: VType,
        v_mounted_here: Option<Weak<Mount>>,
    ) -> Self {
        VNode {
            vfsp,
            v_data_op: data_op,
            v_type,
            v_mounted_here,
            flags: 0,
        }
    }
}

impl VNode {
    pub fn close(&self) {
        self.v_data_op.lock().close();
    }

    pub fn access(&self) {
        self.v_data_op.lock().access()
    }

    pub fn bmap(&self) {
        self.v_data_op.lock().bmap()
    }

    pub fn create(&mut self, path: String, v_type: VType) -> Result<Arc<Spinlock<VNode>>> {
        self.v_data_op.lock().create(path, v_type)
    }

    pub fn fsync(&self) {
        self.v_data_op.lock().fsync()
    }

    pub fn getattr(&self) {
        self.v_data_op.lock().getattr()
    }

    pub fn inactive(&self) {
        self.v_data_op.lock().inactive()
    }

    pub fn ioctl(&self) {
        self.v_data_op.lock().ioctl()
    }

    pub fn link(&self) {
        self.v_data_op.lock().link()
    }

    pub fn lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>> {
        self.v_data_op.lock().lookup(path)
    }

    pub fn mknod(&self) {
        self.v_data_op.lock().mknod()
    }

    pub fn open(&self) {
        self.v_data_op.lock().open()
    }

    pub fn pathconf(&self) {
        self.v_data_op.lock().pathconf()
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.v_data_op.lock().read(buf)
    }

    pub fn readdir(&self) {
        self.v_data_op.lock().readdir()
    }

    pub fn readlink(&self) {
        self.v_data_op.lock().readlink()
    }

    pub fn reclaim(&self) {
        self.v_data_op.lock().reclaim()
    }

    pub fn remove(&self) {
        self.v_data_op.lock().remove()
    }

    pub fn rename(&self) {
        self.v_data_op.lock().rename()
    }

    pub fn mkdir(&self) {
        self.v_data_op.lock().mkdir()
    }

    pub fn rmdir(&self) {
        self.v_data_op.lock().rmdir()
    }

    pub fn setattr(&self) {
        self.v_data_op.lock().setattr()
    }

    pub fn symlink(&self) {
        self.v_data_op.lock().symlink()
    }

    pub fn write(&self, buf: &mut [u8]) -> Result<usize> {
        self.v_data_op.lock().write(buf)
    }

    pub fn kqfilter(&self) {
        self.v_data_op.lock().kqfilter()
    }
}

/// This trait maps logical operations to real functions. It is file system specific as the actions taken by each operation depend heavily on the file system where the file resides.
pub trait VNodeOperations {
    /// Aborts an in-progress operation.
    fn abortop(&self) {
        unimplemented!()
    }

    /// Checks access permissions on a file.
    fn access(&self) {
        unimplemented!()
    }

    fn advlock(&self) {
        unimplemented!()
    }

    /// Maps a logical block number to a physical block number.
    fn bmap(&self) {
        unimplemented!()
    }

    /// Writes a system buffer.
    fn bwrite(&self) {
        unimplemented!()
    }

    /// Closes a file.
    fn close(&self);

    /// Creates a new file.
    fn create(&mut self, path: String, v_type: VType) -> Result<Arc<Spinlock<VNode>>>;

    /// Synchronizes the file with on-disk contents.
    fn fsync(&self) {
        unimplemented!()
    }

    /// Gets a file's attributes.
    fn getattr(&self) {
        unimplemented!()
    }

    /// Marks the vnode as inactive.
    fn inactive(&self) {
        unimplemented!()
    }

    /// Performs an ioctl on a file.
    fn ioctl(&self);

    /// Creates a new hard link for a file.
    fn link(&self) {
        unimplemented!()
    }

    /// Performs a path name lookup.
    fn lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>>;

    /// Creates a new special file (a device or a named pipe).
    fn mknod(&self);

    /// Opens a file.
    fn open(&self);

    /// Returns pathconf information.
    fn pathconf(&self) {
        unimplemented!()
    }

    /// Reads a chunk of data from a file.
    fn read(&self, buf: &mut [u8]) -> Result<usize>;

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
    fn setattr(&self) {
        unimplemented!()
    }

    /// Performs a file transfer between the file system's backing store and memory.
    fn strategy(&self) {
        unimplemented!()
    }

    /// Creates a new symbolic link for a file.
    fn symlink(&self);

    /// Writes a chunk of data to a file.
    fn write(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn kqfilter(&self) {
        unimplemented!()
    }

    fn print(&self) {
        unimplemented!()
    } // OpenBSD has it, NetBSD not?!

    /// Performs a fcntl on a file.
    fn fcntl(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Performs a poll on a file.
    fn poll(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Revoke access to a vnode and all aliases.
    fn revoke(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Maps a file on a memory region.
    fn mmap(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Test and inform file system of seek
    fn seek(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Truncates a file.
    fn truncate(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Updates a file's times.
    fn update(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Reads memory pages from the file.
    fn getpages(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
    /// Writes memory pages to the file.
    fn putpages(&self) {
        unimplemented!()
    } // NetBSD has it, OpenBSD not?!
}
