use crate::current_cpu;
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
    callback: fn(T),
    data: T,
    deadline: usize,
    state: EventState,
    callback_core: u32,
}

impl<T> EventExecutor for TimerEvent<T> {
    fn dispatch(self: Box<Self>) {
        (self.callback)(self.data)
    }

    fn deadline(&self) -> usize {
        self.deadline
    }
}

impl<T> TimerEvent<T> {
    pub fn new(callback: fn(T), data: T, deadline: usize) -> Self {
        Self {
            callback,
            data,
            deadline,
            state: EventState::Waiting,
            callback_core: current_cpu().lapic_id,
        }
    }
}
