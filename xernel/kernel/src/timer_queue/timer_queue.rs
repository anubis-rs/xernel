use alloc::collections::VecDeque;
use crate::timer_queue::timer_event::TimerEvent;
use crate::timer_queue::timer_event::EventExecutor;
use alloc::sync::Arc;

pub struct TimerQueue {
    events: VecDeque<Arc<dyn EventExecutor>>,
    // timer: ???
}

impl TimerQueue {
    
    pub fn event_dispatch(&mut self) {
        if let Some(event) = self.events.pop_front() {
            event.dispatch();
        }
    }

    pub fn queue_event<T: 'static>(&mut self, event: TimerEvent<T>) {
        // FIXME: insert sorted by deadline
        self.events.push_back(Arc::new(event));
    }
}
