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
        let new_task = self.tasks.pop_front().unwrap();

        println!("{:?}", new_task);

        self.tasks.push_front(new_task.clone());

        restore_context(&new_task.context);
    }
}

#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    println!("dont tell me what tod o {:?}", ctx);

    let mut sched = SCHEDULER.lock();

    //sched.save_ctx(ctx);

    APIC.lock().eoi();

    sched.run_next_task();

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
