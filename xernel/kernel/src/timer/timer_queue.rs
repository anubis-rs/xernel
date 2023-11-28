use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::interrupts::allocate_vector;
use crate::arch::amd64::interrupts::ipl::IPL;
use crate::arch::amd64::interrupts::register_handler;
use crate::cpu::current_cpu;
use crate::dbg;
use crate::sched::context::TrapFrame;
use crate::timer::timer_event::EventExecutor;
use crate::timer::timer_event::TimerEvent;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use libxernel::sync::Once;

static TIMER_VECTOR: Once<u8> = Once::new();

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
        if self.events.len() == 0 {
            APIC.oneshot(*TIMER_VECTOR, event.deadline() as u64);
        }

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

    TIMER_VECTOR.set_once(vector.clone());

    register_handler(vector, timer_interrupt_handler);
}

pub fn timer_interrupt_handler(frame: &mut TrapFrame) {
    // get event to fire.
    // create dpc and add to queue
    // if periodic, add again to queue
    // set timer to next event in queue
    //
    debug!("Handler for timer event queue");

    let cpu = current_cpu();
    let mut timer_queue = cpu.timer_queue.write();

    timer_queue.event_dispatch();

    let next_event = timer_queue.events.front();

    if let Some(event) = next_event {

        APIC.oneshot(*TIMER_VECTOR, event.deadline() as u64);

        if event.periodic() {
  //          timer_queue.queue_event(event.clone());
        }

    } else {
        // No event in event queue?
    }

    debug!("ENd of timer event queue handler");
}
