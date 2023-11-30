use crate::{arch::amd64::interrupts::dpc::{Dpc, DpcCall}, current_cpu}; 
use alloc::{boxed::Box, sync::Arc};

pub trait EventExecutor {
    fn dispatch(self);
}

enum EventState {
    Waiting,
    Running,
}

pub struct TimerEvent {
    dpc: Box<dyn DpcCall>,
    //    nanosecs: usize,
    pub deadline: usize,
    state: EventState,
    callback_core: u32,
    pub periodic: bool,
}

impl EventExecutor for TimerEvent {
    fn dispatch(self) {
        current_cpu().dpc_queue.write().add_dpc(self.dpc);
    }
}

impl TimerEvent {
    pub fn new<T: 'static>(callback: fn(T), data: T, deadline: usize, periodic: bool) -> Self {
        let dpc = Dpc::new(callback, data);
        Self {
            dpc: Box::new(dpc),
            deadline,
            state: EventState::Waiting,
            callback_core: current_cpu().lapic_id,
            periodic
        }
    }
}
