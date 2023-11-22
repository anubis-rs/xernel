use alloc::sync::Arc;
use crate::current_cpu;

pub trait EventExecutor {
    fn dispatch(&self);
}

enum EventState {
    Waiting,
    Running
}

pub struct TimerEvent<T> {
    callback: fn(Arc<T>),
    data: Arc<T>,
    deadline: usize,
    state: EventState,
    callback_core: u32,
}

impl<T> EventExecutor for TimerEvent<T> {
    fn dispatch(&self) {
        (self.callback)(self.data.clone())
    }
}

impl<T> TimerEvent<T> {
    pub fn new(callback: fn(Arc<T>), data: T, deadline: usize) -> Self {
        Self {
            callback,
            data: Arc::new(data),
            deadline,
            state: EventState::Waiting,
            callback_core: current_cpu().lapic_id
        }
    }
}
