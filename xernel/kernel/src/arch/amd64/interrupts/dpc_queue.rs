use alloc::{collections::VecDeque, boxed::Box, sync::Arc};

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

    pub fn add_dpc(&mut self, dpc: Box<dyn DpcCall>) {
        self.dpcs.push_front(dpc);
    }

    pub fn work_off(&mut self) {

        for i in self.dpcs.drain(..) {
            i.call();
        }

        println!("after call to schedule");
        
    }
}
