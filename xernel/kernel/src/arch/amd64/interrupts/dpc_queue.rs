use alloc::{collections::VecDeque, boxed::Box};

use super::dpc::{Dpc, DpcCall};

pub struct DpcQueue {
    pub dpcs: VecDeque<Box<dyn DpcCall>>,
}

impl DpcQueue {
    pub fn new() -> Self {
        Self {
            dpcs: VecDeque::new()
        }
    }

    pub fn add_dpc<T: 'static>(&mut self, dpc: Dpc<T>) {
        self.dpcs.push_front(Box::new(dpc));
    }
}
