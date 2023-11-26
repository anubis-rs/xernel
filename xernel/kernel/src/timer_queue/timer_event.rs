use crate::{arch::amd64::interrupts::dpc::Dpc, current_cpu};
use alloc::boxed::Box;

pub trait EventExecutor {
    fn dispatch(self: Box<Self>);
    fn deadline(&self) -> usize;
}

enum EventState {
    Waiting,
    Running,
}

pub struct TimerEvent<T> {
   // callback: fn(T),
   // data: T,
    dpc: Dpc<T>,
    //    nanosecs: usize,
    deadline: usize,
    state: EventState,
    callback_core: u32,
    //    periodic: bool,
}

impl<T: 'static> EventExecutor for TimerEvent<T> {
    fn dispatch(self: Box<Self>) {
        current_cpu().dpc_queue.write().add_dpc(self.dpc);
    }

    fn deadline(&self) -> usize {
        self.deadline
    }
}

impl<T> TimerEvent<T> {
    pub fn new(callback: fn(T), data: T, deadline: usize) -> Self {
        let dpc = Dpc::new(callback, data);
        Self {
            dpc,
            deadline,
            state: EventState::Waiting,
            callback_core: current_cpu().lapic_id,
        }
    }
}
