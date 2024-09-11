use crate::{arch::amd64::{apic::APIC, ports::inb}, sched::context::TrapFrame};

pub fn keyboard(_: &mut TrapFrame) {
    dbg!("keyboard hit");
    let scancode = unsafe { inb(0x60) };
    dbg!("scancode: {}", scancode);
    debug!("scancode: {}", scancode);
    APIC.eoi();
}
