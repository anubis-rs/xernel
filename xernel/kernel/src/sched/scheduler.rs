use crate::arch::x64::gdt::GDT_BSP;
use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, Task};
use alloc::collections::VecDeque;
use libxernel::sync::SpinlockIRQ;
use x86_64::registers::model_specific::KernelGsBase;
use x86_64::registers::segmentation::{Segment, DS};
use x86_64::{PrivilegeLevel, VirtAddr};

use super::context::TaskContext;
use super::task::TaskStatus;

pub struct Scheduler {
    pub tasks: VecDeque<Task>,
}

lazy_static! {
    pub static ref SCHEDULER: SpinlockIRQ<Scheduler> = SpinlockIRQ::new(Scheduler::new());
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    pub fn add_task(&mut self, new_task: Task) {
        self.tasks.push_back(new_task);
    }

    pub fn schedule_task() {}

    pub fn schedule_next_task() {}

    pub fn save_ctx(&mut self, ctx: TaskContext) {
        let mut task = self.tasks.get_mut(0).unwrap();
        task.context = ctx;
    }

    pub fn get_next_task(&mut self) -> &mut Task {
        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        let t = self.tasks.front_mut().unwrap();

        t
    }

    pub fn set_current_task_status(&mut self, status: TaskStatus) {
        let mut task = self.tasks.front_mut().unwrap();
        task.status = status;
    }

    pub fn current_task(&mut self) -> &mut Task {
        self.tasks.front_mut().unwrap()
    }
}

// FIXME: Doesn't work when multiple cores are started
#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    // TODO: Switch page table if user task

    let mut sched = SCHEDULER.lock();
    sched.save_ctx(ctx);

    sched.set_current_task_status(TaskStatus::Waiting);

    let task = sched.get_next_task();

    task.status = TaskStatus::Running;

    if !task.is_kernel_task() {
        unsafe {
            // SAFETY: a user task always has a page table
            // FIXME: a child task must use the page table of the parent task
            task.page_table.as_ref().unwrap().load_pt();

            let mut cs = GDT_BSP.1.user_code_selector;
            let mut ds = GDT_BSP.1.user_data_selector;

            cs.0 |= PrivilegeLevel::Ring3 as u16;
            ds.0 |= PrivilegeLevel::Ring3 as u16;

            DS::set_reg(ds);

            task.context.cs = cs.0 as u64;
            task.context.ss = ds.0 as u64;

            let base = &**task.kernel_stack.as_ref().unwrap() as *const _ as u64;
            KernelGsBase::write(VirtAddr::new(base));
        }
    }

    let context = task.context.clone();

    let mut apic = APIC.lock();
    apic.eoi();
    apic.create_oneshot_timer(0x40, task.priority.ms() * 1000);
    SpinlockIRQ::unlock(apic);

    SpinlockIRQ::unlock(sched);

    restore_context(&context);
}
