use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use libxernel::sync::Spinlock;

use super::{
    mount::{Mount, VfsOps},
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
            .position(|x| x.0 == name_of_fs.to_string())
            .unwrap();

        let driver = self.drivers.get(idx).unwrap().1.clone();

        driver.lock().vfs_mount(where_to_mount.to_string());

        let mut node_covered = if where_to_mount == "/" {
            None
        } else {
            // get vnode to mount on
            Some(self.lookuppn(where_to_mount.to_string()))
        };

        let mount = Mount::new(driver, node_covered);

        self.mount_point_list
            .push((where_to_mount.to_string(), Arc::new(mount)));
    }

    /// Lookup path name
    fn lookuppn(&mut self, path: String) -> Arc<Spinlock<VNode>> {
        // get filesystem path is mounted to
        let mnt = self.mount_point_list.first_mut().unwrap().1.clone();
        let node = mnt.mnt_op_data.lock().vfs_lookup(path);
        node
    }

    pub fn vn_open(&mut self, path: String, mode: u64) {
        let node = self.lookuppn(path);

        node.lock().v_data_op.open();
    }

    pub fn vn_close() {}

    pub fn vn_rdwr() {}

    pub fn vn_create() {}

    pub fn vn_remove() {}

    pub fn vn_link() {}

    pub fn vn_rename() {}
}

pub fn init() {
    let mut vfs = VFS.lock();

    let tmpfs = Arc::new(Spinlock::new(Tmpfs::new()));

    vfs.register_filesystem(String::from("tmpfs"), tmpfs);
    vfs.vn_mount("tmpfs", "/");
}
