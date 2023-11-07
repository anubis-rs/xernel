use alloc::collections::VecDeque;
use crate::time::event::Event;

pub struct Dispatcher<T> {
    event_queue: VecDeque<Event<T>>,
    // timer: ???
}

impl<T> Dispatcher<T> {



}