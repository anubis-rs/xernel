use core::ops::RangeBounds;

use alloc::{boxed::Box, collections::VecDeque};

use super::dpc::DpcCall;

pub struct DpcQueue {
    pub dpcs: VecDeque<Box<dyn DpcCall>>,
}

impl DpcQueue {
    pub fn new() -> Self {
        Self { dpcs: VecDeque::new() }
    }

    pub fn add_dpc(&mut self, dpc: Box<dyn DpcCall>) {
        self.dpcs.push_front(dpc);
    }

    pub fn drain<R>(&mut self, range: R) -> VecDeque<Box<dyn DpcCall>>
    where
        R: RangeBounds<usize>,
    {
        let mut dpcs: VecDeque<Box<dyn DpcCall>> = VecDeque::new();

        self.dpcs.drain(range).for_each(|dpc| dpcs.push_front(dpc));
        dpcs
    }

    pub fn dequeue(&mut self) -> Option<Box<dyn DpcCall>> {
        self.dpcs.pop_front()
    }
}
