use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, Task};
use alloc::collections::VecDeque;
use libxernel::sync::SpinlockIRQ;

use super::context::TaskContext;
use super::task::{TaskPriority, TaskStatus};

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

    pub fn get_next_task(&mut self) -> (TaskPriority, TaskContext) {
        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        let t = self.tasks.front_mut().unwrap();

        (t.priority, t.context.clone())
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
    // TODO: Take priority in account

    let mut sched = SCHEDULER.lock();
    sched.save_ctx(ctx);

    sched.set_current_task_status(TaskStatus::Waiting);

    let (new_priority, new_ctx) = sched.get_next_task();

    sched.set_current_task_status(TaskStatus::Running);

    SpinlockIRQ::unlock(sched);

    let mut apic = APIC.lock();
    apic.eoi();
    apic.create_oneshot_timer(0x40, new_priority.ms() * 1000);
    SpinlockIRQ::unlock(apic);

    restore_context(&new_ctx);
}
