use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use libxernel::sync::Spinlock;

use super::{
    error::{Error, Result},
    mount::{Mount, VfsOps},
    pathbuf::PathBuf,
    tmpfs::Tmpfs,
    vnode::VNode,
};

pub static VFS: Spinlock<Vfs> = Spinlock::new(Vfs::new());

pub struct Vfs {
    mount_point_list: Vec<(PathBuf, Arc<Spinlock<Mount>>)>,
    drivers: Vec<(String, Arc<Spinlock<dyn VfsOps>>)>,
    free_vnodes: Vec<Arc<VNode>>,
}

impl Vfs {
    // get virtual node by asking the file system driver (use mount point list to see which driver to ask)
    // veneer layer gets implemented here
    pub const fn new() -> Self {
        Vfs {
            mount_point_list: Vec::new(),
            drivers: Vec::new(),
            free_vnodes: Vec::new(),
        }
    }

    pub fn get_mount(&self, mounted_on: &PathBuf) -> Result<Arc<Spinlock<Mount>>> {
        self.mount_point_list
            .iter()
            .find(|(pt, _)| pt == mounted_on)
            .map(|(_, mnt)| mnt)
            .ok_or(Error::MountPointNotFound)
            .cloned()
    }

    pub fn register_filesystem(&mut self, name: String, operations: Arc<Spinlock<dyn VfsOps>>) {
        self.drivers.push((name, operations));
    }

    pub fn vn_mount(&mut self, name_of_fs: &str, where_to_mount: &str) -> Result<()> {
        let driver = self
            .drivers
            .iter()
            .find(|(name, _)| name == name_of_fs)
            .map(|(_, driver)| driver)
            .ok_or(Error::FileSystemNotFound)?;

        let node_covered = if where_to_mount == "/" {
            None
        } else {
            // get vnode to mount on
            if let Ok(node) = self.lookuppn(where_to_mount.to_string()) {
                Some(node)
            } else {
                return Err(Error::EntryNotFound);
            }
        };

        let mount = Arc::new(Spinlock::new(Mount::new(driver.clone(), node_covered)));

        let root_node = mount.lock().vfs_root().expect("root node not found");

        root_node.lock().vfsp = Arc::downgrade(&mount);

        mount.lock().vfs_mount(where_to_mount.to_string());

        mount.lock().vfs_start();

        self.mount_point_list
            .push((PathBuf::from(where_to_mount), mount));

        Ok(())
    }

    /// Lookup path name
    pub fn lookuppn(&self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        let path = PathBuf::from(path);

        let mnt_point = self.get_mount_point(&path)?;

        let mnt = self
            .mount_point_list
            .iter()
            .find(|(pt, _)| pt == mnt_point)
            .map(|(_, mnt)| mnt)
            .ok_or(Error::MountPointNotFound)?;

        mnt.lock().vfs_lookup(&path.strip_prefix(mnt_point))
    }

    fn get_mount_point(&self, path: &PathBuf) -> Result<&PathBuf> {
        let mnt_point = self
            .mount_point_list
            .iter()
            .filter(|(pt, _)| path.starts_with(pt))
            .max_by_key(|(pt, _)| pt.len())
            .map(|(pt, _)| pt)
            .ok_or(Error::MountPointNotFound)?;

        Ok(mnt_point)
    }

    pub fn vn_open(&mut self, path: String, _mode: u64) -> Result<Arc<Spinlock<VNode>>> {
        let node = self.lookuppn(path)?;

        node.lock().open();

        Ok(node)
    }

    pub fn vn_close(&mut self) {}

    // TODO: When available, replace node with filedescriptor
    pub fn vn_read(&mut self, node: Arc<Spinlock<VNode>>, buf: &mut [u8]) -> Result<usize> {
        node.lock().read(buf)
    }

    pub fn vn_write(&mut self, node: Arc<Spinlock<VNode>>, buf: &mut [u8]) -> Result<usize> {
        node.lock().write(buf)
    }

    pub fn vn_create(&mut self) {}

    pub fn vn_remove(&mut self) {}

    pub fn vn_link(&mut self) {}

    pub fn vn_rename(&mut self) {}
}

pub fn init() {
    let mut vfs = VFS.lock();

    let tmpfs = Arc::new(Spinlock::new(Tmpfs::new()));

    tmpfs.lock().vfs_init();

    vfs.register_filesystem(String::from("tmpfs"), tmpfs);

    vfs.vn_mount("tmpfs", "/")
        .expect("Mounting tmpfs on / failed");
}
