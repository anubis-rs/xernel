use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    arch::x64::{apic::APIC, ports::inb},
    dbg, debug,
};

pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    dbg!("keyboard hit");
    let mut apic = APIC.lock();
    let scancode = inb(0x60);
    dbg!("scancode: {}", scancode);
    debug!("scancode: {}", scancode);
    apic.eoi();
}
