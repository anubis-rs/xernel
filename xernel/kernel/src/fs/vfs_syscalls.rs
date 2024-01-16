use alloc::string::String;

use crate::{syscall::Result, cpu::current_process};

use super::{file::File, vfs::VFS};

pub fn sys_open(path: String, mode: u64) -> Result<isize> {
    let vfs = VFS.lock();

    let node = vfs.vn_open(path, mode)?;

    let file_handle = File::new(node);

    let process = current_process();
    let mut process = process.lock();

    let fd = process.append_fd(file_handle);

    Ok(fd as isize)
}

pub fn sys_close(fd: usize) -> Result<isize> {
    let process = current_process();
    let process = process.lock();

    let file_handle = process.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    node.lock().close();

    Ok(0)
}

pub fn sys_read(fd: usize, buf: &mut [u8]) -> Result<isize> {
    let vfs = VFS.lock();

    let process = current_process();
    let process = process.lock();

    let file_handle = process.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    let res = vfs.vn_read(node, buf)?;

    Ok(res as isize)
}

pub fn sys_write(fd: usize, buf: &mut [u8]) -> Result<isize> {
    let vfs = VFS.lock();

    let process = current_process();
    let process = process.lock();

    let file_handle = process.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    let res = vfs.vn_write(node, buf)?;

    Ok(res as isize)
}
