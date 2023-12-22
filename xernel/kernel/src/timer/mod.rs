use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use crate::cpu::current_cpu;

use self::timer_event::TimerEvent;

pub mod timer_event;
pub mod timer_queue;

static UPTIME: AtomicUsize = AtomicUsize::new(0);

pub fn hardclock(_: ()) {
    println!("hardclock event with uptime {:?}", UPTIME);
    UPTIME.fetch_add(1, Ordering::SeqCst);
    let event = TimerEvent::new(hardclock, (), Duration::from_secs(1), false);

    current_cpu().timer_queue.write().queue_event(event);
}
