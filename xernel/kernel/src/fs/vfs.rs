use alloc::rc::Rc;
use alloc::string::String;
use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use crate::debug;
use crate::fs::error::{Error, Result};

use super::ramfs::RamFs;
use super::VFS;
use super::{mountpoint::FsMountpoint, FsNode, FsNodeHandler};

pub fn init() {
    let mut vfs = VFS.lock();

    vfs.init();
}

pub struct Vfs {
    drivers: BTreeMap<String, Rc<dyn FsNodeHandler>>,
    mount_point_list: Vec<FsMountpoint>,
}

impl Vfs {
    pub const fn new() -> Self {
        Vfs {
            drivers: BTreeMap::new(),
            mount_point_list: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        let ramfs: Rc<dyn FsNodeHandler> = Rc::new(RamFs::new());

        self.install_fs("ramfs".to_string(), ramfs.clone());

        self.mount("/".to_string(), "ramfs".to_string());
    }

    pub fn read_fs(&self) {}

    pub fn write_fs(&self) {}

    pub fn open_fs(&self, filename: &str, flags: u32, mode: u32) -> Result<Rc<FsNode>> {
        debug!("[vfs] open_fs: opening file: {}", filename);

        let mut node: Option<Rc<FsNode>> = None;

        for i in self.mount_point_list.iter() {
            if i.path == filename.to_string() {
                node = Some(i.node.clone());
            }
        }

        if node.is_none() {
            return Err(Error::NodeNotFound);
        }

        let node = node.unwrap();

        node.handler.open();

        Ok(node)
    }

    pub fn close_fs() {}

    pub fn install_fs(&mut self, name: String, handler: Rc<dyn FsNodeHandler>) {
        self.drivers.insert(name, handler);
    }

    pub fn mount(&mut self, source: String, fs_identifier: String) {
        let fs_handler = self.drivers.get(&fs_identifier).unwrap().clone();

        let mount_point = FsMountpoint::new(source, "/".to_string(), fs_handler);
        self.mount_point_list.push(mount_point);
    }
}
