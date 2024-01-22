use core::ops::RangeBounds;

use alloc::{boxed::Box, collections::VecDeque};

use crate::{
    arch::amd64::interrupts::ipl::{raise_ipl, set_ipl, IPL},
    cpu::current_cpu,
    sched::scheduler::switch_threads,
};

pub trait DpcCall {
    fn call(self: Box<Self>);
}

pub enum DpcState {
    DPCUnbound,
    DPCBound,
    DPCRunning,
}

pub struct Dpc<T> {
    pub callback: fn(T),
    pub arg: T,
    state: DpcState,
}

pub struct DpcQueue {
    pub dpcs: VecDeque<Box<dyn DpcCall>>,
}


impl<T> DpcCall for Dpc<T> {
    fn call(self: Box<Self>) {
        (self.callback)(self.arg)
    }
}

impl<T> Dpc<T> {
    pub fn new(callback: fn(T), data: T) -> Self {
        Self {
            callback,
            arg: data,
            state: DpcState::DPCUnbound,
        }
    }
}

impl DpcQueue {
    pub fn new() -> Self {
        Self { dpcs: VecDeque::new() }
    }

    pub fn enqueue(&mut self, dpc: Box<dyn DpcCall>) {
        self.dpcs.push_back(dpc);
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

pub fn enqueue_dpc(dpc: Box<dyn DpcCall>) {
    current_cpu().dpc_queue.write().enqueue(dpc)
}

pub fn dpc_interrupt_dispatch() {
    let cpu = current_cpu();

    let ipl = raise_ipl(IPL::IPLDPC);

    while let Some(dpc) = {
        let old = raise_ipl(IPL::IPLHigh);
        let mut lock = cpu.dpc_queue.write();
        let dpc = lock.dequeue();
        set_ipl(old);
        dpc
    } {
        dpc.call();
    }

    set_ipl(ipl);

    let old = cpu.current_thread.read().clone();
    let new = cpu.next.read().clone();

    if old.is_some() && new.is_some() {
        *cpu.next.write() = None;
        switch_threads(old.unwrap(), new.unwrap());
    }
}
