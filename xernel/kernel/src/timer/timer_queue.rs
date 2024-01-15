use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::interrupts::allocate_vector;
use crate::arch::amd64::interrupts::ipl::IPL;
use crate::arch::amd64::interrupts::register_handler;
use crate::arch::amd64::tsc;
use crate::cpu::current_cpu;
use crate::sched::context::TrapFrame;
use crate::timer::timer_event::EventExecutor;
use crate::timer::timer_event::TimerEvent;
use alloc::collections::VecDeque;
use libxernel::sync::Once;

static TIMER_VECTOR: Once<u8> = Once::new();

pub struct TimerQueue {
    events: VecDeque<TimerEvent>,
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
        if let Some(event) = self.events.pop_front() {
            self.events.iter_mut().for_each(|i| i.deadline -= event.deadline);

            event.dispatch();
        }
    }

    pub fn queue_event(&mut self, event: TimerEvent) {
        if self.events.len() == 0 {
            println!("hey yo, im set");
            APIC.oneshot(*TIMER_VECTOR, (event.deadline) as u64);
        }

        let insert_index = self
            .events
            .iter()
            .position(|i| i.deadline >= event.deadline)
            .unwrap_or(self.events.len());
        self.events.insert(insert_index, event);

        self.events.iter().for_each(|i| println!("event deadline: {}", i.deadline));
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

pub fn init() {

    tsc::calibrate_tsc();

    let vector = allocate_vector(IPL::IPLClock).expect("Could not allocate vector for timer interrupt");

    TIMER_VECTOR.set_once(vector.clone());

    info!("TIMER_VECTOR initialized to: {}", *TIMER_VECTOR);

    register_handler(vector, timer_interrupt_handler);
}

pub fn timer_interrupt_handler(frame: &mut TrapFrame) {
    // if periodic, add again to queue
    // set timer to next event in queue
    
    debug!("timer_interrupt_handler");

    let cpu = current_cpu();
    let mut timer_queue = cpu.timer_queue.write(); 

    debug!("timer_queue_len: {}", timer_queue.events.len());
    debug!("dpc_queue_len: {}", cpu.dpc_queue.read().dpcs.len());
    timer_queue.event_dispatch();

    debug!("timer_queue_len: {}", timer_queue.events.len());
    debug!("dpc_queue_len: {}", cpu.dpc_queue.read().dpcs.len());

    let next_event = timer_queue.events.front();

    if let Some(event) = next_event {
        println!("setting timer to next event with vec {}", *TIMER_VECTOR);
        APIC.oneshot(*TIMER_VECTOR, (event.deadline) as u64);

        if event.periodic {
            //timer_queue.queue_event(event.clone());
        }

    } else {
        // No event in event queue?
    }

    debug!("End of timer event queue handler");
}
