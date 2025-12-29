use crate::arch::amd64::apic::APIC;
use crate::timer::TIMER_VECTOR;
use crate::timer::timer_event::EventExecutor;
use crate::timer::timer_event::TimerEvent;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::time::Duration;

pub struct TimerQueue {
    pub events: VecDeque<TimerEvent>,
    pub is_timer_set: bool,
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            is_timer_set: false,
        }
    }

    pub fn event_dispatch(&mut self) {
        let mut deadline = Duration::ZERO;

        if let Some(event) = self.events.pop_front() {
            deadline = event.deadline;
            event.dispatch();
        }

        let mut indices_to_remove: Vec<usize> = Vec::new();

        for (index, ev) in self.events.iter_mut().enumerate() {
            ev.deadline -= deadline;

            if ev.deadline.is_zero() {
                indices_to_remove.push(index);
            }
        }

        for &index in indices_to_remove.iter().rev() {
            if let Some(event) = self.events.remove(index) {
                event.dispatch();
            }
        }
    }

    pub fn enqueue(&mut self, event: TimerEvent) {
        if self.events.is_empty() {
            APIC.oneshot(*TIMER_VECTOR, &event.deadline);
            self.events.push_front(event);
        } else {
            if event.deadline < self.events.front().unwrap().deadline {
                APIC.stop();
                APIC.oneshot(*TIMER_VECTOR, &event.deadline);
            }

            let insert_index = self
                .events
                .iter()
                .position(|i| i.deadline >= event.deadline)
                .unwrap_or(self.events.len());
            self.events.insert(insert_index, event);
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn deadlines(&self) {
        println!("===");
        self.events
            .iter()
            .for_each(|i| println!("event deadline: {:?}", i.deadline));

        if let Some(event) = self.events.front() {
            let first = event.deadline;
            if self.events.iter().all(|ev| ev.deadline == first) {
                println!("all have the same deadline");
            }
        }
    }
}
