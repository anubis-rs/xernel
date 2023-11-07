use alloc::sync::Arc;

enum EventState {
    Waiting,
    Running
}

pub struct Event<T> {
    callback: fn(Arc<T>),
    data: Arc<T>,
    deadline: usize,
    state: EventState
}

impl<T> Event<T> {
    fn trigger(&self) {
        (self.callback)(self.data.clone());
    }
}