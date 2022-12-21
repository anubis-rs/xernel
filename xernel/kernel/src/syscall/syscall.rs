use crate::{fs::VFS, sched::scheduler::SCHEDULER};

pub fn sys_open(filename: &str, flags: u32, mode: u32) -> u32 {
    let vfs = VFS.lock();
    let mut scheduler = SCHEDULER.lock();

    let node = vfs.open_fs(filename, flags, mode);

    let mut current_task = scheduler.current_task();

    let fd = current_task.append_fd(node.unwrap());
    return fd;
}
