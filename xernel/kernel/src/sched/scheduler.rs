use crate::acpi::hpet;
use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::gdt::GDT_BSP;
use crate::cpu::{get_per_cpu_data, PerCpu, CPU_COUNT};
use crate::sched::context::restore_context;
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
use x86_64::structures::idt::InterruptStackFrame;
use crate::arch::{allocate_vector, register_handler};

use super::context::ThreadContext;
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

        let isr = allocate_vector();

        register_handler(isr, schedule_handle);

        SCHEDULER_VECTOR.set_once(isr);

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
        let mut schedulers = unsafe {
            SCHEDULER
                .get_all()
                .iter()
                .map(|s| s.lock())
                .collect::<Vec<_>>()
        };

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

                while tasks_needed > 0
                    && !schedulers[j].threads.is_empty()
                    && schedulers[j].threads.len() > avg_tasks
                {
                    let task = schedulers[j].threads.back().unwrap().lock();

                    if task.status == ThreadStatus::Running {
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

    pub fn save_ctx(&mut self, ctx: ThreadContext) {
        let mut task = self.threads.get_mut(0).unwrap().lock();
        task.context = ctx;
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
        self.threads.front_mut().unwrap().lock().status = status;
    }

    fn executing_thread(&mut self) -> Arc<Spinlock<Thread>> {
        {
            let task = self.threads.front();

            if let Some(task) = task {
                if task.lock().status != ThreadStatus::Running {
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

        APIC.create_oneshot_timer(*SCHEDULER_VECTOR, 1);

        interrupts::enable();

        unsafe {
            asm!("hlt");
        }
    }
}

#[naked]
pub extern "C" fn scheduler_irq_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        asm!(
            "push r15;
            push r14; 
            push r13;
            push r12;
            push r11;
            push r10;
            push r9;
            push r8;
            push rdi;
            push rsi;
            push rdx;
            push rcx;
            push rbx;
            push rax;
            push rbp;
            call schedule_handle",
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn schedule_handle(ctx: ThreadContext) {
    let mut sched = SCHEDULER.get().lock();
    if let Some(task) = sched.threads.get(0) && task.lock().status == ThreadStatus::Running {
        sched.save_ctx(ctx);

        sched.set_current_thread_status(ThreadStatus::Ready);
    }

    let thread = sched.get_next_thread().unwrap_or(sched.idle_thread.clone());
    let mut thread = thread.lock();

    thread.status = ThreadStatus::Running;

    if !thread.is_kernel_thread() {
        unsafe {
            // SAFETY: a user thread always has a page table
            let pt = thread
                .process
                .upgrade()
                .unwrap()
                .lock()
                .get_page_table()
                .unwrap();

            let cr3 = Cr3::read_raw();
            let cr3 = cr3.0.start_address().as_u64() | cr3.1 as u64;

            // Only reload the page table if it's different to avoid unnecessary TLB flushes
            if cr3 != pt.pml4().as_u64() {
                pt.load_pt();
            }

            DS::set_reg(GDT_BSP.1.user_data_selector);

            get_per_cpu_data()
                .set_kernel_stack(thread.kernel_stack.as_ref().unwrap().kernel_stack_top);
        }
    }

    let context = &thread.context as *const ThreadContext;

    APIC.eoi();
    APIC.create_oneshot_timer(*SCHEDULER_VECTOR, thread.priority.ms() * 1000);

    thread.unlock();
    sched.unlock();

    restore_context(context);
}
