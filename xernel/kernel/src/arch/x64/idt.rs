#![feature(abi_x86_interrupt)]
use lazy_static::lazy_static;
use x86_64::set_general_handler;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use core::arch::asm;

use crate::{print, println};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        set_general_handler!(&mut idt, interrupt_handler);
        idt
    };
}

pub fn init() {
    IDT.load();
}

fn interrupt_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    println!("IP: {:?}", stack_frame.instruction_pointer);
    println!("index: {}", index);
    println!("error_code: {}", error_code.unwrap_or(0));
    unsafe {
        asm!("hlt");
    }
}
