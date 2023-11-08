use alloc::sync::Arc;

trait EventExecutor {
    fn dispatch(&self);
}

enum EventState {
    Waiting,
    Running
}

pub struct Event<T> {
    callback: fn(Arc<T>),
    data: Arc<T>,
    deadline: usize,
    state: EventState,
    callback_core: usize,
}

impl<T> EventExecutor for Event<T> {
    fn dispatch(&self) {
        (self.callback)(self.data.clone())
    }
}

impl<T> Event<T> {}
