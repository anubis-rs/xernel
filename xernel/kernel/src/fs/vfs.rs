use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use libxernel::sync::Spinlock;

use super::{
    mount::{Mount, VfsOps},
    vnode::VNode,
};

pub static VFS: Spinlock<Vfs> = Spinlock::new(Vfs::new());

pub struct Vfs {
    mount_point_list: Vec<(String, Arc<Mount>)>,
    drivers: Vec<(String, Arc<dyn VfsOps>)>,
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

    pub fn register_filesystem(&mut self, name: String, operations: Arc<dyn VfsOps>) {
        self.drivers.push((name, operations));
    }

    pub fn vn_mount(&mut self, name_of_fs: &str, where_to_mount: &str) {
        // return if driver for given fs is not registered
        let idx = self
            .drivers
            .iter()
            .position(|x| x.0 == name_of_fs.to_string())
            .unwrap();

        let driver = self.drivers.get(idx).unwrap().1.clone();

        // TODO: Give path (where fs should be mounted to) to fs_mount
        driver.fs_mount();

        // TODO: check if mounted on root node, else get reference to node where it gets mounted on
        // since when root node, node_covered is null
        let mut node_covered = None;

        let mount = Mount::new(driver, node_covered);

        self.mount_point_list
            .push((where_to_mount.to_string(), Arc::new(mount)));
    }

    pub fn vn_lookuppn() {}

    pub fn vn_open() {}

    pub fn vn_close() {}

    pub fn vn_rdwr() {}

    pub fn vn_create() {}

    pub fn vn_remove() {}

    pub fn vn_link() {}

    pub fn vn_rename() {}
}

pub fn init() {
    let mut vfs = VFS.lock();

    //vfs.register_filesystem(String::from("tmpfs"), tmpfs);
    vfs.vn_mount("tmpfs", "/");
}
