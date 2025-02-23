use alloc::sync::Arc;
use libxernel::sync::Spinlock;

use crate::{
    cpu::current_cpu,
    fs::initramfs::initramfs_read,
    sched::{
        process::{Process, KERNEL_PROCESS},
        thread::Thread,
    },
};

pub fn init() {
    let init_elf = initramfs_read("init").expect("init process not found in initramfs");
    let init_process = Arc::new(Spinlock::new(Process::new(Some(KERNEL_PROCESS.clone()))));
    KERNEL_PROCESS.lock().children.push(init_process.clone());

    let entry_point = init_process.aquire().load_elf(&init_elf);

    dbg!("init process entry point: {:#x}", entry_point);

    let init_thread = Thread::new_user_thread(init_process.clone(), entry_point);
    current_cpu().enqueue_thread(Arc::new(init_thread));
}
