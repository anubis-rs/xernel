use crate::arch::amd64::gdt::GDT_BSP;
use crate::arch::amd64::switch_context;
use crate::cpu::current_cpu;
use crate::timer::timer_event::TimerEvent;
use alloc::sync::Arc;
use core::time::Duration;
use x86_64::registers::control::Cr3;
use x86_64::registers::segmentation::{Segment, DS};

use super::thread::{Thread, ThreadStatus};

pub fn reschedule(_: ()) {
    let cpu = current_cpu();

    let next_ref = cpu.run_queue.write().pop_front();

    let current_ref = cpu.current_thread.read().clone();

    let old = if let Some(current_thread) = current_ref {
        current_thread.clone()
    } else {
        *cpu.current_thread.write() = Some(cpu.idle_thread.clone());
        cpu.idle_thread.clone()
    };

    let new = if let Some(next_thread) = next_ref {
        cpu.run_queue.write().push_back(next_thread.clone());

        next_thread.clone()
    } else {
        cpu.idle_thread.clone()
    };

    register_reschedule_event(new.priority.ms());

    if Arc::ptr_eq(&old, &new) {
        return;
    }

    *cpu.next.write() = Some(new);
}

pub fn enqueue_thread(thread: Thread) {
    current_cpu().run_queue.write().push_back(Arc::new(thread));
}

pub fn dequeue_thread(thread: Arc<Thread>) -> Option<Arc<Thread>> {
    let cpu = current_cpu();

    let mut index_to_remove = 0;

    for (i, thrd) in cpu.run_queue.write().iter().enumerate() {
        if Arc::ptr_eq(&thread, thrd) {
            index_to_remove = i;
            break;
        }
    }

    let thread = cpu.run_queue.write().remove(index_to_remove);
    thread
}

pub fn switch_threads(old: Arc<Thread>, new: Arc<Thread>) {
    old.status.set(ThreadStatus::Ready);

    new.status.set(ThreadStatus::Running);

    if !new.is_kernel_thread() {
        unsafe {
            let process = new.process.upgrade().unwrap();
            let mut process = process.lock();

            // SAFETY: A user thread always has a page table
            let pt = process.get_page_table().as_mut().unwrap();

            let cr3 = Cr3::read_raw();

            let cr3 = cr3.0.start_address().as_u64() | cr3.1 as u64;

            if cr3 != pt.pml4().as_u64() {
                pt.load_pt();
            }

            DS::set_reg(GDT_BSP.1.user_data_selector);

            current_cpu()
                .kernel_stack
                .set(new.kernel_stack.as_ref().unwrap().kernel_stack_top);
        }
    }

    *current_cpu().current_thread.write() = Some(new.clone());

    unsafe {
        switch_context(old.context.get(), *new.context.get());
    }
}

fn register_reschedule_event(millis: u64) {
    let event = TimerEvent::new(reschedule, (), Duration::from_millis(millis), false);

    let cpu = current_cpu();
    let mut timer_queue = cpu.timer_queue.write();

    timer_queue.enqueue(event);
}
