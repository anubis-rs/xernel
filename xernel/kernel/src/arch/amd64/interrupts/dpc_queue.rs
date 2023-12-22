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

    pub fn work_off(&mut self) -> VecDeque<Box<dyn DpcCall>> {
        let mut dpcs: VecDeque<Box<dyn DpcCall>> = VecDeque::new();

        self.dpcs.drain(..).for_each(|dpc| dpcs.push_front(dpc));
        dpcs
    }
}
