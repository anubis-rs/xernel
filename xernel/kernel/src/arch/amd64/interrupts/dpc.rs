use alloc::boxed::Box;

use crate::{
    arch::amd64::interrupts::ipl::{raise_spl, set_ipl, IPL},
    cpu::{current_cpu, PerCpu},
    sched::scheduler::switch_threads,
};

pub static DPC_VECTOR: PerCpu<u8> = PerCpu::new();

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

pub fn dpc_interrupt_dispatch() {
    let cpu = current_cpu();

    let ipl = raise_spl(IPL::IPLDPC);

    while let Some(dpc) = {
        let old = raise_spl(IPL::IPLHigh);
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
