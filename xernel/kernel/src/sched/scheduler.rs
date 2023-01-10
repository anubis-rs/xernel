use crate::arch::x64::gdt::GDT_BSP;
use crate::cpu::{get_per_cpu_data, PerCpu};
use crate::sched::context::restore_context;
use crate::{arch::x64::apic::APIC, Task};
use alloc::collections::VecDeque;
use core::arch::asm;
use libxernel::sync::SpinlockIRQ;
use x86_64::registers::control::Cr3;
use x86_64::registers::segmentation::{Segment, DS};
use x86_64::structures::idt::InterruptStackFrame;

use super::context::TaskContext;
use super::task::TaskStatus;

pub struct Scheduler {
    pub tasks: VecDeque<Task>,
}

pub static SCHEDULER: PerCpu<SpinlockIRQ<Scheduler>> = PerCpu::new();

impl Scheduler {
    pub const fn new() -> Self {
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
        let task = self.tasks.get_mut(0).unwrap();
        task.context = ctx;
    }

    pub fn get_next_task(&mut self) -> &mut Task {
        let old_task = self.tasks.pop_front().unwrap();

        self.tasks.push_back(old_task);

        let t = self.tasks.front_mut().unwrap();

        t
    }

    pub fn set_current_task_status(&mut self, status: TaskStatus) {
        let task = self.tasks.front_mut().unwrap();
        task.status = status;
    }

    pub fn current_task(&mut self) -> &mut Task {
        self.tasks.front_mut().unwrap()
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
    sched.save_ctx(ctx);

    sched.set_current_task_status(TaskStatus::Waiting);

    let task = sched.get_next_task();

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

    let context = task.context.clone();

    APIC.eoi();
    APIC.create_oneshot_timer(0x40, task.priority.ms() * 1000);

    SpinlockIRQ::unlock(sched);

    restore_context(&context);
}
