use core::arch::asm;
use libxernel::spin::Spinlock;
use x86_64::registers::control::Cr2;
use x86_64::set_general_handler;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::arch::x64::ports::outb;
use x86_64::structures::idt::HandlerFunc;

use crate::{print, println};

static IDT: Spinlock<InterruptDescriptorTable> = Spinlock::new(InterruptDescriptorTable::new());

pub fn init() {
    let mut idt = IDT.lock();

    set_general_handler!(&mut idt, interrupt_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);

    unsafe { idt.load_unsafe(); }
}

pub fn set_handler(entry: usize, handler: HandlerFunc) {
    let mut idt = IDT.lock();

    idt[entry].set_handler_fn(handler);
}

fn interrupt_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    println!("IP: {:?}", stack_frame.instruction_pointer);
    println!("index: {}", index);
    println!("error_code: {}", error_code.unwrap_or(0));
    unsafe {
        asm!("hlt");
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

/// Disable Programmable Interrupt Controller.
pub fn disable_pic() {
    // Set ICW1
    outb(0x20, 0x11);
    outb(0xa0, 0x11);

    // Set IWC2 (IRQ base offsets)
    outb(0x21, 0xe0);
    outb(0xa1, 0xe8);

    // Set ICW3
    outb(0x21, 4);
    outb(0xa1, 2);

    // Set ICW4
    outb(0x21, 1);
    outb(0xa1, 1);

    // Set OCW1 (interrupt masks)
    outb(0x21, 0xff);
    outb(0xa1, 0xff);
}
