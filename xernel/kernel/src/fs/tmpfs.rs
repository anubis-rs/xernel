use alloc::{
    string::{String, ToString},
    sync::Arc,
    sync::Weak,
    vec::Vec,
};
use libxernel::{boot::InitAtBoot, sync::Spinlock};

use crate::{
    fs::error::{Error, Result},
    println,
};

use super::{
    mount::VfsOps,
    vnode::{VNode, VNodeOperations, VType},
};

pub struct Tmpfs {
    root_node: InitAtBoot<Arc<Spinlock<VNode>>>,
    mounted_on: Option<String>,
}

impl Tmpfs {
    pub fn new() -> Self {
        Self {
            root_node: InitAtBoot::Uninitialized,
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
        let mut node = TmpfsNode::new(VType::Regular);

        let data = node.data.as_mut().unwrap();

        data.push(0xFE);
        data.push(0xFF);
        data.push(0xFF);

        self.root_node.lock().create(
            "/test.txt".to_string(),
            Arc::new(Spinlock::new(VNode::new(
                Weak::new(),
                Arc::new(Spinlock::new(node)),
                VType::Regular,
                None,
            ))),
        );
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

    fn vfs_init(&mut self) {
        let tmpfs_node = TmpfsNode::new(VType::Directory);

        let root = Arc::new(Spinlock::new(VNode::new(
            Weak::new(),
            Arc::new(Spinlock::new(tmpfs_node)),
            VType::Directory,
            None,
        )));

        self.root_node = InitAtBoot::Initialized(root);
    }

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
    fn vfs_lookup(&self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        println!("tmpfs path lookup: {}", path);

        let node = self.root_node.lock().lookup(path);

        node
    }
}

pub struct TmpfsNode {
    data: Option<Vec<u8>>,
    children: Option<Vec<(String, Arc<Spinlock<VNode>>)>>,
    vtype: VType,
}

impl TmpfsNode {
    pub fn new(vtype: VType) -> Self {
        if vtype == VType::Directory {
            Self {
                data: None,
                children: Some(Vec::new()),
                vtype,
            }
        } else {
            Self {
                data: Some(Vec::new()),
                children: None,
                vtype,
            }
        }
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

    fn create(&mut self, path: String, node: Arc<Spinlock<VNode>>) -> Result<()> {
        if self.vtype != VType::Directory {
            return Err(Error::NotADirectory);
        }

        self.children.as_mut().unwrap().push((path, node));

        Ok(())
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

    fn lookup(&self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        println!("tmpfs path lookup: {}", path);

        if self.vtype != VType::Directory {
            return Err(Error::NotADirectory);
        } else {
            for child in self.children.as_ref().unwrap() {
                if child.0 == path {
                    return Ok(child.1.clone());
                }
            }
        }

        Err(Error::EntryNotFound)
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
