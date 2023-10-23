use x86_64::structures::idt::InterruptStackFrame;

use crate::arch::amd64::{apic::APIC, ports::inb};

pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    dbg!("keyboard hit");
    let scancode = inb(0x60);
    dbg!("scancode: {}", scancode);
    debug!("scancode: {}", scancode);
    APIC.eoi();
}
