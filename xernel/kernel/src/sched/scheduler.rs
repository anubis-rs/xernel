use crate::acpi::hpet;
use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::gdt::GDT_BSP;
use crate::arch::amd64::switch_context;
use crate::arch::{allocate_vector, register_handler};
use crate::cpu::{current_cpu, PerCpu, CPU_COUNT};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use libxernel::sync::{Once, Spinlock, SpinlockIRQ};
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr3;
use x86_64::registers::segmentation::{Segment, DS};

use super::context::TrapFrame;
use super::process::Process;
use super::thread::{Thread, ThreadStatus};

pub struct Scheduler {
    pub threads: VecDeque<Arc<Spinlock<Thread>>>,
    pub idle_thread: Arc<Spinlock<Thread>>,
}

pub static SCHEDULER: PerCpu<SpinlockIRQ<Scheduler>> = PerCpu::new();

pub static SCHEDULER_VECTOR: Once<u8> = Once::new();

impl Scheduler {
    pub fn new() -> Self {
        Self {
            threads: VecDeque::new(),
            idle_thread: Arc::new(Spinlock::new(Thread::new_idle_thread())),
        }
    }

    pub fn current_thread() -> Arc<Spinlock<Thread>> {
        SCHEDULER.get().lock().executing_thread()
    }

    pub fn current_process() -> Arc<Spinlock<Process>> {
        Self::current_thread().lock().get_process().unwrap()
    }

    pub fn add_thread(&mut self, new_task: Arc<Spinlock<Thread>>) {
        self.threads.push_back(new_task);
    }

    /// Adds the task to the scheduler with the least amount of tasks
    pub fn add_thread_balanced(new_task: Arc<Spinlock<Thread>>) {
        Self::load_balance();

        let mut smallest_queue_index = 0;
        let mut smallest_queue_len = usize::MAX;

        for i in 0..*CPU_COUNT {
            let sched = unsafe { SCHEDULER.get_index(i).lock() };

            if sched.threads.len() < smallest_queue_len {
                smallest_queue_len = sched.threads.len();
                smallest_queue_index = i;
            }
        }

        let mut sched = unsafe { SCHEDULER.get_index(smallest_queue_index).lock() };
        sched.add_thread(new_task);
    }

    /// Balances the load of all schedulers
    /// Currently this method tries to move the tasks between schedulers so every scheduler has the same amount of tasks
    pub fn load_balance() {
        static LAST_LOAD_BALANCE: AtomicU64 = AtomicU64::new(0);

        let now = hpet::milliseconds();
        let diff = now - LAST_LOAD_BALANCE.load(Ordering::Relaxed);

        // only balance every 5 seconds because this function is very slow
        if diff < 5000 {
            return;
        }

        // TODO: currently we lock all schedulers during the load balancing procedure
        //       find a way to avoid locking all schedulers
        let mut schedulers = unsafe { SCHEDULER.get_all().iter().map(|s| s.lock()).collect::<Vec<_>>() };

        let mut total_tasks = 0;

        for sched in &schedulers {
            total_tasks += sched.threads.len();
        }

        let avg_tasks = total_tasks / schedulers.len();

        for i in 0..schedulers.len() {
            let mut tasks_needed = avg_tasks as isize - schedulers[i].threads.len() as isize;

            // move the neededs tasks from the other schedulers to this scheduler
            for j in 0..schedulers.len() {
                if i == j {
                    continue;
                }

                while tasks_needed > 0 && !schedulers[j].threads.is_empty() && schedulers[j].threads.len() > avg_tasks {
                    let task = schedulers[j].threads.back().unwrap().lock();

                    if task.status.get() == ThreadStatus::Running {
                        continue;
                    }

                    task.unlock();

                    let task = schedulers[j].threads.pop_back().unwrap();

                    schedulers[i].threads.push_back(task);
                    tasks_needed -= 1;
                }
            }
        }
    }

    pub fn get_next_thread(&mut self) -> Option<Arc<Spinlock<Thread>>> {
        if self.threads.is_empty() {
            return None;
        }

        let old_task = self.threads.pop_front().unwrap();

        self.threads.push_back(old_task);

        let t = self.threads.front_mut().unwrap();

        Some(t.clone())
    }

    pub fn set_current_thread_status(&mut self, status: ThreadStatus) {
        self.threads.front_mut().unwrap().lock().status.set(status);
    }

    fn executing_thread(&mut self) -> Arc<Spinlock<Thread>> {
        {
            let task = self.threads.front();

            if let Some(task) = task {
                if task.lock().status.get() != ThreadStatus::Running {
                    panic!("current task not running");
                }
            } else {
                panic!("no task executed");
            }
        }
        self.threads.front_mut().unwrap().clone()
    }

    pub fn hand_over() {
        interrupts::disable();

        APIC.oneshot(*SCHEDULER_VECTOR, 1);

        interrupts::enable();

        loop {
            unsafe {
                asm!("hlt");
            }
        }
    }
}

pub fn schedule(_ctx: &mut TrapFrame) {
    // Add new event to EventQueue

    let cpu = current_cpu();

    let next_ref = cpu.run_queue.write().pop_front();

    let current_ref = cpu.current_thread.read().clone();

    let old = if let Some(current_thread) = current_ref {
        current_thread.clone()
    } else {
        cpu.idle_thread.clone()
    };

    old.status.set(ThreadStatus::Ready);

    let new = if let Some(next_thread) = next_ref {
        cpu.run_queue.write().push_back(next_thread.clone());

        *cpu.current_thread.write() = Some(next_thread.clone());

        let status = cpu.current_thread.read().clone().unwrap().status.get();

        APIC.eoi();
        APIC.oneshot(*SCHEDULER_VECTOR, next_thread.priority.ms() * 1000);

        next_thread.clone()
    } else {
        cpu.idle_thread.clone()
    };

    new.status.set(ThreadStatus::Running);

    unsafe {
        switch_context(old.context.get(), *new.context.get());
    }
}

fn switch_threads() {}

pub fn init() {
    if !SCHEDULER_VECTOR.is_completed() {
        let vector = allocate_vector();
        SCHEDULER_VECTOR.set_once(vector);
        register_handler(vector, schedule);
    }

    SCHEDULER.init(|| SpinlockIRQ::new(Scheduler::new()));
    SCHEDULER.wait_until_initialized();
}
