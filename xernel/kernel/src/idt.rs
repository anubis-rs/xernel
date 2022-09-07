use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{println, print};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt
    };
}

pub fn init() {
    IDT.load();
}