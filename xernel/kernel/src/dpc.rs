use core::ops::RangeBounds;

use alloc::{boxed::Box, collections::VecDeque};
use libxernel::ipl::{get_ipl, raise_ipl, splx, IPL};

use crate::{
    arch::amd64::{apic::APIC, write_cr8},
    cpu::current_cpu,
    sched::{context::TrapFrame, scheduler::switch_threads},
};

pub trait DpcCall {
    fn call(self: Box<Self>);
}

pub enum DpcState {
    Unbound,
    Bound,
    Running,
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
            state: DpcState::Unbound,
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
    if get_ipl() < IPL::DPC {
        let ipl = raise_ipl(IPL::DPC);

        log!("calling dpc directly");
        dpc.call();

        splx(ipl);
        return;
    }

    current_cpu().dpc_queue.write().enqueue(dpc);
    raise_dpc_interrupt()
}

pub fn raise_dpc_interrupt() {
    log!("raise dpc");
    warning!("{:?}", get_ipl());
    APIC.send_ipi(current_cpu().lapic_id, 0x2f)
}

pub fn dispatch_dpcs(_: &mut TrapFrame) {
    log!("working off dpcs");
    let cpu = current_cpu();

    assert!(get_ipl() == IPL::DPC);

    while let Some(dpc) = {
        let old = raise_ipl(IPL::High);
        let mut lock = cpu.dpc_queue.write();
        let dpc = lock.dequeue();
        write_cr8(old);
        dpc
    } {
        dpc.call();
    }

    let old = cpu.current_thread.read().clone();
    let new = cpu.next.read().clone();

    debug!("switching to thread");

    if old.is_some() && new.is_some() {
        *cpu.next.write() = None;
        let ipl = get_ipl();
        switch_threads(old.unwrap(), new.unwrap());
        splx(ipl);
    }
}
