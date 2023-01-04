use alloc::{
    rc::Weak,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use libxernel::sync::Spinlock;

use crate::println;

use super::{
    mount::VfsOps,
    vnode::{VNode, VNodeOperations},
};

pub struct Tmpfs {
    files: Vec<(String, Arc<Spinlock<VNode>>)>,
    mounted_on: Option<String>,
}

impl Tmpfs {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            mounted_on: None,
        }
    }
}

impl VfsOps for Tmpfs {
    fn vfs_mount(&mut self, path: String) {
        println!("mounting tmpfs on {}", path);

        self.mounted_on = Some(path)
    }

    fn vfs_start(&mut self) {
        let mut node = TmpfsNode::new();

        node.data.push(0xFF);
        node.data.push(0xFF);
        node.data.push(0xFF);

        self.files.push((
            "/test.txt".to_string(),
            Arc::new(Spinlock::new(VNode::new(
                Weak::new(),
                Arc::new(node),
                crate::fs::vnode::VType::Regular,
                None,
            ))),
        ));
    }

    fn vfs_unmount(&self) {
        todo!()
    }

    fn vfs_root(&self) {
        todo!()
    }

    fn vfs_quotactl(&self) {
        todo!()
    }

    fn vfs_statvfs(&self) {
        todo!()
    }

    fn vfs_sync(&self) {
        todo!()
    }

    fn vfs_vget(&self) {
        todo!()
    }

    fn vfs_fhtovp(&self) {
        todo!()
    }

    fn vfs_vptofh(&self) {
        todo!()
    }

    fn vfs_init(&mut self) {}

    fn vfs_done(&self) {
        todo!()
    }

    fn vfs_extattrctl(&self) {
        todo!()
    }

    fn vfs_name(&self) -> String {
        "tmpfs".to_string()
    }

    // FIXME: Write a proper lookup algorithm, which gets a directory node and calls lookup on that
    fn vfs_lookup(&self, path: String) -> Arc<Spinlock<VNode>> {
        println!("tmpfs path lookup: {}", path);

        for i in &self.files {
            if i.0 == path {
                return i.1.clone();
            }
        }

        return self.files.first().unwrap().1.clone();
    }
}

pub struct TmpfsNode {
    data: Vec<u8>,
}

impl TmpfsNode {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}

impl VNodeOperations for TmpfsNode {
    fn access(&self) {
        todo!()
    }

    fn bmap(&self) {
        todo!()
    }

    fn close(&self) {
        todo!()
    }

    fn create(&self) {
        todo!()
    }

    fn fsync(&self) {
        todo!()
    }

    fn getattr(&self) {
        todo!()
    }

    fn inactive(&self) {
        todo!()
    }

    fn ioctl(&self) {
        todo!()
    }

    fn link(&self) {
        todo!()
    }

    fn lookup(&self) {
        todo!()
    }

    fn mknod(&self) {
        todo!()
    }

    fn open(&self) {
        println!("opening file on tmpfs");
    }

    fn pathconf(&self) {
        todo!()
    }

    fn read(&self) {
        println!("{:?}", self.data);
    }

    fn readdir(&self) {
        todo!()
    }

    fn readlink(&self) {
        todo!()
    }

    fn reclaim(&self) {
        todo!()
    }

    fn remove(&self) {
        todo!()
    }

    fn rename(&self) {
        todo!()
    }

    fn mkdir(&self) {
        todo!()
    }

    fn rmdir(&self) {
        todo!()
    }

    fn setattr(&self) {
        todo!()
    }

    fn symlink(&self) {
        todo!()
    }

    fn write(&self) {
        todo!()
    }

    fn kqfilter(&self) {
        todo!()
    }
}
