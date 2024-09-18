pub mod timer_event;
pub mod timer_queue;

use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use crate::{arch::amd64::interrupts::allocate_vector, cpu::current_cpu};

use self::timer_event::TimerEvent;

use crate::amd64::interrupts::register_handler;
use crate::amd64::tsc;
use crate::apic::APIC;
use crate::sched::context::TrapFrame;
use libxernel::{ipl::IPL, sync::Once};

static UPTIME: AtomicUsize = AtomicUsize::new(0);
static TIMER_VECTOR: Once<u8> = Once::new();

pub fn init() {
    tsc::calibrate_tsc();

    if let Some(vec) = allocate_vector(IPL::Clock) {
        TIMER_VECTOR.set_once(vec);
    } else {
        panic!("Could not allocate timer vector");
    }

    register_handler(*TIMER_VECTOR, timer_interrupt_handler);
}

pub fn timer_interrupt_handler(_frame: &mut TrapFrame) {
    // if periodic, add again to queue
    // set timer to next event in queue

    let cpu = current_cpu();

    let mut timer_queue = cpu.timer_queue.aquire_at(IPL::High);

    //timer_queue.deadlines();

    timer_queue.event_dispatch();

    let next_event = timer_queue.events.front();

    if let Some(event) = next_event {
        APIC.oneshot(*TIMER_VECTOR, &event.deadline);

        // TODO: Find a way to clone event
        if event.periodic {
            //timer_queue.queue_event(event.clone());
        }
    } else {
        // No event in event queue?
    }
}

pub fn hardclock(_: ()) {
    println!("hardclock event with uptime {:?}", UPTIME);
    UPTIME.fetch_add(1, Ordering::SeqCst);
    let event = TimerEvent::new(hardclock, (), Duration::from_secs(1), false);

    current_cpu().enqueue_timer(event);
}
