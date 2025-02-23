use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use libxernel::sync::Spinlock;

use crate::utils::limine_module;

static initramfs: Spinlock<BTreeMap<String, Vec<u8>>> = Spinlock::new(BTreeMap::new());

pub fn load_initramfs() {
    let file = limine_module::get_limine_module("initramfs").unwrap();
    let data = unsafe { core::slice::from_raw_parts(file.base.as_ptr().unwrap(), file.length as usize) };

    let mut idx: usize = 0;

    while idx < file.length as usize {
        let name = String::from_utf8(data[idx..idx + 16].iter().take_while(|&&b| b != 0).copied().collect())
            .expect("Invalid UTF-8 in the name of a file in initramfs");
        idx += 16;

        let size = u64::from_le_bytes([
            data[idx],
            data[idx + 1],
            data[idx + 2],
            data[idx + 3],
            data[idx + 4],
            data[idx + 5],
            data[idx + 6],
            data[idx + 7],
        ]) as usize;
        idx += 8;

        let file_data = data[idx..idx + size].to_vec();
        idx += size;

        initramfs.lock().insert(name, file_data);
    }
}

pub fn initramfs_read(path: &str) -> Option<Vec<u8>> {
    initramfs.lock().get(&path.to_string()).cloned()
}
