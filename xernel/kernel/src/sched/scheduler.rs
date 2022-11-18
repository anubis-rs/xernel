use crate::dbg;
use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, println, Task};
use alloc::collections::VecDeque;
use libxernel::spin::Spinlock;
use libxernel::ticket::TicketMutex;

use super::context::TaskContext;
use super::task::TaskStatus;

pub struct Scheduler {
    pub tasks: VecDeque<Task>,
}

lazy_static! {
    pub static ref SCHEDULER: Spinlock<Scheduler> = Spinlock::new(Scheduler::new());
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

    pub fn save_ctx(&mut self, ctx: TaskContext) {
        // FIXME: Plain unwrap, use if let
        let mut task = self.tasks.get_mut(0).unwrap();
        task.context = ctx;
    }

    pub fn get_next_task(&mut self) -> &Task {
        // FIXME: Plain unwrap, use if let
        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        // FIXME: Plain unwrap, use if let
        self.tasks.front_mut().unwrap()
    }

    pub fn set_current_task_status(&mut self, status: TaskStatus) {
        // FIXME: Plain unwrap, use if let
        let mut task = self.tasks.front_mut().unwrap();
        task.status = status;
    }
}

#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    // TODO: Switch page table if user task
    // TODO: Take priority in account
    // TODO: Change TaskStatus accordingly

    let mut sched = SCHEDULER.lock();
    sched.save_ctx(ctx);

    sched.set_current_task_status(TaskStatus::Waiting);

    x86_64::instructions::interrupts::enable();

    let new_ctx = sched.get_next_task().context.clone();
    Spinlock::unlock(sched);

    let mut apic = APIC.lock();
    apic.eoi();
    TicketMutex::unlock(apic);

    dbg!("restoring context");

    restore_context(&new_ctx);
}
