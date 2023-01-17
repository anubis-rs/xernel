use crate::arch::x64::gdt::GDT_BSP;
use crate::cpu::{get_per_cpu_data, PerCpu};
use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, Task};
use alloc::collections::VecDeque;
use core::arch::asm;
use libxernel::sync::SpinlockIRQ;
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr3;
use x86_64::registers::segmentation::{Segment, DS};
use x86_64::structures::idt::InterruptStackFrame;

use super::context::TaskContext;
use super::task::TaskStatus;

pub struct Scheduler {
    pub tasks: VecDeque<Task>,
    pub idle_task: Task,
}

pub static SCHEDULER: PerCpu<SpinlockIRQ<Scheduler>> = PerCpu::new();

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            idle_task: Task::new_idle_task(),
        }
    }

    pub fn add_task(&mut self, new_task: Task) {
        self.tasks.push_back(new_task);
    }

    pub fn schedule_task() {}

    pub fn schedule_next_task() {}

    pub fn save_ctx(&mut self, ctx: TaskContext) {
        let task = self.tasks.get_mut(0).unwrap();
        task.context = ctx;
    }

    pub fn get_next_task(&mut self) -> Option<&mut Task> {
        if self.tasks.is_empty() {
            return None;
        }

        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        let t = self.tasks.front_mut().unwrap();

        Some(t)
    }

    pub fn set_current_task_status(&mut self, status: TaskStatus) {
        let task = self.tasks.front_mut().unwrap();
        task.status = status;
    }

    pub fn current_task(&mut self) -> &mut Task {
        self.tasks.front_mut().unwrap()
    }

    pub fn hand_over() {
        interrupts::disable();

        APIC.create_oneshot_timer(0x40, 1);

        interrupts::enable();

        unsafe {
            asm!("hlt");
        }

        unreachable!();
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
pub extern "sysv64" fn schedule_handle(ctx: TaskContext) {
    let mut sched = SCHEDULER.get().lock();
    if let Some(task) = sched.tasks.get(0) && task.status == TaskStatus::Running {
        sched.save_ctx(ctx);

        sched.set_current_task_status(TaskStatus::Waiting);
    }

    let task = match sched.get_next_task() {
        Some(t) => t,
        None => &mut sched.idle_task, // Use the idle task if there are no other tasks to schedule
    };

    task.status = TaskStatus::Running;

    if !task.is_kernel_task() {
        unsafe {
            // SAFETY: a user task always has a page table
            let pt = task.get_page_table().unwrap();

            let cr3 = Cr3::read_raw();
            let cr3 = cr3.0.start_address().as_u64() | cr3.1 as u64;

            // Only reload the page table if it's different to avoid unnecessary TLB flushes
            if cr3 != pt.pml4().as_u64() {
                pt.load_pt();
            }

            DS::set_reg(GDT_BSP.1.user_data_selector);

            get_per_cpu_data().set_kernel_stack(task.kernel_stack.as_ref().unwrap().end as usize);
        }
    }

    let context = &task.context as *const TaskContext;

    APIC.eoi();
    APIC.create_oneshot_timer(0x40, task.priority.ms() * 1000);

    sched.unlock();

    restore_context(context);
}
