use crate::debug;

use super::{
    error::{Error, Result},
    FsNodeHandler,
};

struct RamFsFile {
    
}

pub struct RamFs {



}

impl FsNodeHandler for RamFs {
    fn read(&self, buf: &mut [u8], count: usize, offset: usize) -> Result<usize> {
        Ok(12)
    }

    fn write(&self, buf: &mut [u8], count: usize, offset: usize) -> usize {
        12
    }

    fn open(&self) {
        debug!("open in ramfs");
    }

    fn close(&self) {}

    fn readdir(&self) {}

    fn finddir(&self) {}

    fn create(&self) {}

    fn mkdir(&self) {}
}

impl RamFs {
    pub const fn new() -> Self {
        RamFs {}
    }
}
