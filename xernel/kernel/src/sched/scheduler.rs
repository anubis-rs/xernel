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
    pub static ref SCHEDULER: Spinlock<Scheduler> = {
        let tm = Spinlock::new(Scheduler::new());
        tm
    };
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
        let mut task = self.tasks.get_mut(0).unwrap();
        task.context = ctx;
    }

    pub fn get_next_task(&mut self) -> &Task {
        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task.clone());

        self.tasks.get(0).unwrap()
    }

    pub fn set_current_task_waiting(&mut self) {
        let mut task = self.tasks.get_mut(0).unwrap();
        task.status = TaskStatus::Waiting;
    }
}

#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    println!("test");
    let mut sched = SCHEDULER.lock();
    sched.save_ctx(ctx);

    sched.set_current_task_waiting();

    let mut apic = APIC.lock();
    apic.eoi();
    TicketMutex::unlock(apic);

    x86_64::instructions::interrupts::enable();

    let new_task = sched.get_next_task().clone();
    Spinlock::unlock(sched);

    println!("{:?}", new_task.context);
    dbg!("restoring context");
    restore_context(&new_task.context);
}
