use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use libxernel::sync::Spinlock;

use crate::{debug, println};

use super::{
    error::{Error, Result},
    mount::{Mount, VfsOps},
    pathbuf::PathBuf,
    tmpfs::Tmpfs,
    vnode::VNode,
};

pub static VFS: Spinlock<Vfs> = Spinlock::new(Vfs::new());

pub struct Vfs {
    mount_point_list: Vec<(String, Arc<Mount>)>,
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

    pub fn register_filesystem(&mut self, name: String, operations: Arc<Spinlock<dyn VfsOps>>) {
        self.drivers.push((name, operations));
    }

    pub fn vn_mount(&mut self, name_of_fs: &str, where_to_mount: &str) {
        // TODO: return if driver for given fs is not registered
        let idx = self
            .drivers
            .iter()
            .position(|x| x.0 == *name_of_fs.to_string());

        if idx.is_none() {
            return; // TODO: return error here
        }

        let driver = self.drivers.get(idx.unwrap()).unwrap().1.clone();

        let node_covered = if where_to_mount == "/" {
            None
        } else {
            // get vnode to mount on
            if let Ok(node) = self.lookuppn(where_to_mount.to_string()) {
                Some(node)
            } else {
                return;
            }
        };

        let mut mount = Mount::new(driver, node_covered);

        mount.vfs_mount(where_to_mount.to_string());

        mount.vfs_start();

        self.mount_point_list
            .push((where_to_mount.to_string(), Arc::new(mount)));
    }

    /// Lookup path name
    pub fn lookuppn(&mut self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        // TODO: get filesystem path is mounted to
        let path = PathBuf::from(path);

        let mnt_point = self.get_mount_point(&path);

        if mnt_point.is_err() {
            return Err(Error::MountPointNotFound);
        }

        let mnt_point_name = mnt_point.unwrap();

        let mnt = &self
            .mount_point_list
            .iter()
            .find(|s| s.0 == mnt_point_name)
            .unwrap()
            .1;

        let mnt = &self.mount_point_list.first().unwrap().1;

        mnt.vfs_lookup(path)
    }

    fn get_mount_point(&mut self, path: &PathBuf) -> Result<String> {
        let components = path.components();

        Ok(String::from("/"))
    }

    pub fn vn_open(&mut self, path: String, mode: u64) -> Result<()> {
        let node = self.lookuppn(path);

        node?.lock().open();

        Ok(())
    }

    pub fn vn_close(&mut self) {}

    pub fn vn_read(&mut self, path: String) -> Result<()> {
        let node = self.lookuppn(path);

        node?.lock().read();

        Ok(())
    }

    pub fn vn_write(&mut self) {}

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

    vfs.vn_mount("tmpfs", "/");
}
