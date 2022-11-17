use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, println, Task};
use alloc::collections::VecDeque;
use core::arch::asm;

use libxernel::ticket::TicketMutex;

use super::context::TaskContext;

pub struct Scheduler {
    tasks: VecDeque<Task>,
}

lazy_static! {
    pub static ref SCHEDULER: TicketMutex<Scheduler> = {
        let tm = TicketMutex::new(Scheduler::new());
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

    pub fn run_next_task(&mut self) {
        let old_task = self.tasks.pop_front().unwrap();

        println!("{:?}", old_task);

        self.tasks.push_back(old_task.clone());

        let new_task = self.tasks.get(0).unwrap();

        println!("{:?}", new_task);

        restore_context(&new_task.context);
    }
}

#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    println!("test");
    let mut sched = SCHEDULER.lock();

    sched.save_ctx(ctx);

    let mut apic = APIC.lock();
    apic.eoi();

    sched.run_next_task();

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
