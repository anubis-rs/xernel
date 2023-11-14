use alloc::collections::VecDeque;
use crate::time::event::EventExecutor;
use alloc::sync::Arc;

pub struct EventQueue {
    events: VecDeque<Arc<dyn EventExecutor>>,
    // timer: ???
}

impl EventQueue {
    
    pub fn event_dispatch(&mut self) {
        let event = self.events.pop_front().unwrap();
        event.dispatch();
    }
    
}
