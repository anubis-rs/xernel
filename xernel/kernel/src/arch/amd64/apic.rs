use alloc::vec::Vec;
use libxernel::sync::{Once, Spinlock};
use x86_64::structures::idt::InterruptStackFrame;

use crate::arch::amd64::{ioapic};
use crate::arch::amd64::ioapic::IOApic;
use crate::arch::amd64::lapic::LocalApic;
use crate::sched::context::TrapFrame;

pub struct LocalAPIC {
    address: u64,
    frequency: u64,
}

pub static IOAPICS: Spinlock<Vec<IOApic>> = Spinlock::new(Vec::new());

pub static APIC: Once<LocalApic> = Once::new();

pub fn init() {
    let mut io_apics = IOAPICS.lock();

    ioapic::init(&mut *io_apics);

    APIC.set_once(LocalApic::new());
}

pub fn apic_spurious_interrupt(_stack_frame: TrapFrame) {
    APIC.eoi();
}
