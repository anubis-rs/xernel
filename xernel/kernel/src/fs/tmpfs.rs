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

        let data = if let TmpfsNodeData::Data(data) = &mut node.data {
            data
        } else {
            return;
        };

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

    fn vfs_vget(&self) {
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

    fn vfs_name(&self) -> String {
        "tmpfs".to_string()
    }

    // FIXME: Write a proper lookup algorithm, which gets a directory node and calls lookup on that
    fn vfs_lookup(&self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        println!("tmpfs path lookup: {}", path);

        let node = self.root_node.lock().lookup(path);

        node
    }

    fn vfs_sync(&self) {
        todo!()
    }
}

enum TmpfsNodeData {
    Children(Vec<(String, Arc<Spinlock<VNode>>)>),
    Data(Vec<u8>),
}

pub struct TmpfsNode {
    data: TmpfsNodeData,
}

impl TmpfsNode {
    pub fn new(vtype: VType) -> Self {
        if vtype == VType::Directory {
            Self {
                data: TmpfsNodeData::Children(Vec::new()),
            }
        } else {
            Self {
                data: TmpfsNodeData::Data(Vec::new()),
            }
        }
    }
}

impl VNodeOperations for TmpfsNode {
    fn close(&self) {
        todo!()
    }

    fn create(&mut self, path: String, node: Arc<Spinlock<VNode>>) -> Result<()> {
        if let TmpfsNodeData::Children(children) = &mut self.data {
            children.push((path, node));
        } else {
            return Err(Error::NotADirectory);
        }

        Ok(())
    }

    fn ioctl(&self) {
        todo!()
    }

    fn lookup(&self, path: String) -> Result<Arc<Spinlock<VNode>>> {
        println!("tmpfs path lookup: {}", path);

        if let TmpfsNodeData::Children(children) = &self.data {
            for child in children {
                if child.0 == path {
                    return Ok(child.1.clone());
                }
            }
        } else {
            return Err(Error::NotADirectory);
        }

        Err(Error::EntryNotFound)
    }

    fn mknod(&self) {
        todo!()
    }

    fn open(&self) {
        println!("opening file on tmpfs");
    }

    fn read(&self) {
        if let TmpfsNodeData::Data(data) = &self.data {
            println!("reading tmpfs node: {:?}", data);
        } else {
            println!("have to throw error");
        }
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

    fn symlink(&self) {
        todo!()
    }

    fn write(&self) {
        todo!()
    }
}
