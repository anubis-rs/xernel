use alloc::boxed::Box;
use libxernel::sync::Once;

use crate::{
    cpu::{current_cpu, PerCpu},
    sched::context::TrapFrame,
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

pub fn dpc_interrupt_dispatch(_frame: &mut TrapFrame) {
    let cpu = current_cpu();

    let mut dpcs = cpu.dpc_queue.write().work_off();

    dpcs.drain(..).for_each(|dpc| dpc.call());
}
