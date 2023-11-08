use alloc::collections::VecDeque;
use crate::time::event::Event;
use crate::time::event::EventExecutor;

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
