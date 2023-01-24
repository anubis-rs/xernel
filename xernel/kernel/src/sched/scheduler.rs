use crate::acpi::hpet;
use crate::arch::x64::apic::APIC;
use crate::arch::x64::gdt::GDT_BSP;
use crate::cpu::{get_per_cpu_data, PerCpu, CPU_COUNT};
use crate::sched::context::restore_context;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use libxernel::sync::{Spinlock, SpinlockIRQ};
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr3;
use x86_64::registers::segmentation::{Segment, DS};
use x86_64::structures::idt::InterruptStackFrame;

use super::context::ThreadContext;
use super::thread::{Thread, ThreadStatus};

pub struct Scheduler {
    pub tasks: VecDeque<Arc<Spinlock<Thread>>>,
    pub idle_thread: Arc<Spinlock<Thread>>,
}

pub static SCHEDULER: PerCpu<SpinlockIRQ<Scheduler>> = PerCpu::new();

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            idle_thread: Arc::new(Spinlock::new(Thread::new_idle_thread())),
        }
    }

    pub fn current_thread() -> Arc<Spinlock<Thread>> {
        SCHEDULER.get().lock().executing_thread()
    }

    pub fn add_thread(&mut self, new_task: Arc<Spinlock<Thread>>) {
        self.tasks.push_back(new_task);
    }

    /// Adds the task to the scheduler with the least amount of tasks
    pub fn add_thread_balanced(new_task: Arc<Spinlock<Thread>>) {
        Self::load_balance();

        let mut smallest_queue_index = 0;
        let mut smallest_queue_len = usize::MAX;

        for i in 0..*CPU_COUNT {
            let sched = unsafe { SCHEDULER.get_index(i).lock() };

            if sched.tasks.len() < smallest_queue_len {
                smallest_queue_len = sched.tasks.len();
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
            total_tasks += sched.tasks.len();
        }

        let avg_tasks = total_tasks / schedulers.len();

        for i in 0..schedulers.len() {
            let mut tasks_needed = avg_tasks as isize - schedulers[i].tasks.len() as isize;

            // move the neededs tasks from the other schedulers to this scheduler
            for j in 0..schedulers.len() {
                if i == j {
                    continue;
                }

                while tasks_needed > 0
                    && !schedulers[j].tasks.is_empty()
                    && schedulers[j].tasks.len() > avg_tasks
                {
                    let task = schedulers[j].tasks.back().unwrap().lock();

                    if task.status == ThreadStatus::Running {
                        continue;
                    }

                    drop(task);

                    let task = schedulers[j].tasks.pop_back().unwrap();

                    schedulers[i].tasks.push_back(task);
                    tasks_needed -= 1;
                }
            }
        }
    }

    pub fn save_ctx(&mut self, ctx: ThreadContext) {
        let mut task = self.tasks.get_mut(0).unwrap().lock();
        task.context = ctx;
    }

    pub fn get_next_thread(&mut self) -> Option<Arc<Spinlock<Thread>>> {
        if self.tasks.is_empty() {
            return None;
        }

        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        let t = self.tasks.front_mut().unwrap();

        Some(t.clone())
    }

    pub fn set_current_thread_status(&mut self, status: ThreadStatus) {
        self.tasks.front_mut().unwrap().lock().status = status;
    }

    fn executing_thread(&mut self) -> Arc<Spinlock<Thread>> {
        self.tasks.front_mut().unwrap().clone()
    }

    pub fn hand_over() {
        interrupts::disable();

        APIC.create_oneshot_timer(0x40, 1);

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

// TODO: Schedule on multiple cores if multiple cores are started up
#[no_mangle]
pub extern "sysv64" fn schedule_handle(ctx: ThreadContext) {
    let mut sched = SCHEDULER.get().lock();
    if let Some(task) = sched.tasks.get(0) && task.lock().status == ThreadStatus::Running {
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
    APIC.create_oneshot_timer(0x40, thread.priority.ms() * 1000);

    sched.unlock();

    restore_context(context);
}
