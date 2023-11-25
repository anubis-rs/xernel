use crate::arch::amd64::interrupts::allocate_vector;
use crate::arch::amd64::interrupts::ipl::IPL;
use crate::arch::amd64::interrupts::register_handler;
use crate::cpu::current_cpu;
use crate::sched::context::TrapFrame;
use crate::timer_queue::timer_event::EventExecutor;
use crate::timer_queue::timer_event::TimerEvent;
use alloc::boxed::Box;
use alloc::collections::VecDeque;

pub struct TimerQueue {
    events: VecDeque<Box<dyn EventExecutor>>,
    // timer: ???
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn event_dispatch(&mut self) {
        if let Some(event) = self.events.pop_front() {
            event.dispatch();
        }
    }

    pub fn queue_event<T: 'static>(&mut self, event: TimerEvent<T>) {
        let insert_index = self
            .events
            .iter()
            .position(|i| i.deadline() >= event.deadline())
            .unwrap_or(self.events.len());
        self.events.insert(insert_index, Box::new(event));
    }
}

pub fn init() {

    let vector = allocate_vector(IPL::IPLClock).expect("Could not allocate vector for timer interrupt");

    register_handler(vector, timer_interrupt_handler);

}

pub fn timer_interrupt_handler(frame: &mut TrapFrame) {
    
    // get event to fire.
    // create dpc and add to queue
    // if periodic, add again to queue
    // set timer to next event in queue
   
    let cpu = current_cpu();
    let mut timer_queue = cpu.timer_queue.write();

    timer_queue.event_dispatch();

}
