use alloc::string::String;

use crate::{sched::scheduler::SCHEDULER, syscall::Result};

use super::{file::FileHandle, vfs::VFS};

pub fn sys_open(path: String, mode: u64) -> Result<isize> {
    let vfs = VFS.lock();

    let node = vfs.vn_open(path, mode)?;

    let file_handle = FileHandle::new(node);

    let fd = SCHEDULER.get().lock().current_task().append_fd(file_handle);

    Ok(fd as isize)
}

pub fn sys_close(fd: usize) -> Result<isize> {
    let mut scheduler = SCHEDULER.get().lock();

    let task = scheduler.current_task();

    let file_handle = task.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    node.lock().close();

    Ok(0)
}

pub fn sys_read(fd: usize, buf: &mut [u8]) -> Result<isize> {
    let vfs = VFS.lock();

    let mut scheduler = SCHEDULER.get().lock();

    let task = scheduler.current_task();

    let file_handle = task.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    let res = vfs.vn_read(node, buf)?;

    Ok(res as isize)
}

pub fn sys_write(fd: usize, buf: &mut [u8]) -> Result<isize> {
    let vfs = VFS.lock();

    let mut scheduler = SCHEDULER.get().lock();

    let task = scheduler.current_task();

    let file_handle = task.get_filehandle_from_fd(fd);

    let node = file_handle.get_node();

    let res = vfs.vn_write(node, buf)?;

    Ok(res as isize)
}
