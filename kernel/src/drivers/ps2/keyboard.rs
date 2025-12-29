use alloc::boxed::Box;

use crate::{
    arch::amd64::ports::inb,
    dpc::{Dpc, enqueue_dpc},
    sched::context::TrapFrame,
};

pub fn keyboard_handler(_: &mut TrapFrame) {
    let dpc = Dpc::new(keyboard, ());

    enqueue_dpc(Box::new(dpc));
}

pub fn keyboard<T>(_: T) {
    dbg!("keyboard hit");
    let scancode = unsafe { inb(0x60) };
    dbg!("scancode: {}", scancode);
    debug!("scancode: {}", scancode);
}
