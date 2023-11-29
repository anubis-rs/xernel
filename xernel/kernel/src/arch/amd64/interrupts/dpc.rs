use alloc::{boxed::Box, sync::Arc};
use libxernel::sync::Once;

use crate::{sched::context::TrapFrame, cpu::current_cpu, dbg};

pub static DPC_VECTOR: Once<u8> = Once::new();

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

pub fn dpc_interrupt_dispatch(frame: &mut TrapFrame) {
    debug!("in dpc interrupt dispatch");
    let cpu = current_cpu();

    cpu.dpc_queue.write().work_off();
}
