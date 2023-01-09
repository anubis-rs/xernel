use core::{mem, ptr::copy};

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
    pathbuf::PathBuf,
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

        self.root_node
            .lock()
            .create(
                "test.txt".to_string(),
                Arc::new(Spinlock::new(VNode::new(
                    Weak::new(),
                    Arc::new(Spinlock::new(node)),
                    VType::Regular,
                    None,
                ))),
            )
            .expect("Creation of root node in tmpfs failed");
    }

    fn vfs_unmount(&self) {
        todo!()
    }

    fn vfs_root(&self) -> Result<Arc<Spinlock<VNode>>> {
        Ok(self.root_node.clone())
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

    fn vfs_lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>> {
        if path == "/" || path.is_empty() {
            return Ok(self.root_node.clone());
        }

        self.root_node.lock().lookup(path)
    }

    fn vfs_sync(&self) {
        todo!()
    }
}

enum TmpfsNodeData {
    Children(Vec<(PathBuf, Arc<Spinlock<VNode>>)>),
    Data(Vec<u8>),
}

pub struct TmpfsNode {
    parent: Option<Arc<Spinlock<VNode>>>,
    data: TmpfsNodeData,
}

impl TmpfsNode {
    pub fn new(vtype: VType) -> Self {
        if vtype == VType::Directory {
            Self {
                parent: None,
                data: TmpfsNodeData::Children(Vec::new()),
            }
        } else {
            Self {
                parent: None,
                data: TmpfsNodeData::Data(Vec::new()),
            }
        }
    }
}

impl VNodeOperations for TmpfsNode {
    fn close(&self) {
        todo!()
    }

    fn create(&mut self, file_name: String, node: Arc<Spinlock<VNode>>) -> Result<()> {
        if let TmpfsNodeData::Children(children) = &mut self.data {
            children.push((PathBuf::from(file_name), node));
        } else {
            return Err(Error::NotADirectory);
        }

        Ok(())
    }

    fn ioctl(&self) {
        todo!()
    }

    fn lookup(&self, path: &PathBuf) -> Result<Arc<Spinlock<VNode>>> {
        println!("tmpfs path lookup: {}", path);

        let stripped_path = if path.starts_with(&PathBuf::from("/")) {
            path.strip_prefix(&PathBuf::from("/"))
        } else {
            path.clone()
        };

        let components = stripped_path.components();

        if let TmpfsNodeData::Children(children) = &self.data {
            if components.len() == 1 {
                let node = children
                    .iter()
                    .find(|(pt, _)| pt == components[0])
                    .map(|(_, node)| node.clone());
                return node.ok_or(Error::EntryNotFound);
            } else if components.len() > 1 {
                let node = children
                    .iter()
                    .find(|(pt, _)| pt == components[0])
                    .map(|(_, node)| node.clone());

                if let Some(node) = node {
                    return node.lock().lookup(&stripped_path);
                } else {
                    return Err(Error::EntryNotFound);
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

    fn read(&self, buf: &mut [u8]) -> Result<usize> {
        if let TmpfsNodeData::Data(data) = &self.data {
            let max_read = if buf.len() > data.len() {
                data.len()
            } else {
                buf.len()
            };

            buf[..max_read].copy_from_slice(&data[..max_read]);

            Ok(max_read)
        } else {
            return Err(Error::IsADirectory);
        }
    }

    fn write(&mut self, buf: &mut [u8]) -> Result<usize> {
        if let TmpfsNodeData::Data(ref mut data) = &mut self.data {
            let max_write = if buf.len() > data.len() {
                data.len()
            } else {
                buf.len()
            };

            data.reserve(max_write);

            data[..max_write].copy_from_slice(&buf[..max_write]);

            Ok(max_write)
        } else {
            return Err(Error::IsADirectory);
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
}
