use crate::arch::x64::apic::{apic_spurious_interrupt, timer};
use crate::arch::x64::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::arch::x64::ports::outb;
use crate::drivers::ps2::keyboard::keyboard;
use core::arch::asm;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::{set_general_handler, VirtAddr};

use crate::{backtrace, dbg, println};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        set_general_handler!(&mut idt, interrupt_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault
            .set_handler_fn(general_fault_handler);

        unsafe {
            idt[0x40].set_handler_addr(VirtAddr::new(timer as u64));
        }
        idt[0x47].set_handler_fn(keyboard);
        idt[0xff].set_handler_fn(apic_spurious_interrupt);
        idt
    };
}

pub fn init() {
    IDT.load();
}

fn interrupt_handler(stack_frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
    let mut rbp: usize;
    unsafe {
        asm!("mov {}, rbp", out(reg) rbp);
    }

    dbg!("EXCEPTION: {}", index);
    dbg!("{:x?}", stack_frame);

    backtrace::log_backtrace(rbp);

    println!("IP: {:?}", stack_frame.instruction_pointer);
    println!("index: {}", index);
    println!("error_code: {}", error_code.unwrap_or(0));
    unsafe {
        asm!("hlt");
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    dbg!("EXCEPTION: DOUBLE FAULT");
    dbg!("{:#?}", stack_frame);
    dbg!("{}", error_code);
    println!("EXCEPTION: DOUBLE FAULT");
    println!("{:#?}", stack_frame);
    println!("{}", error_code);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    dbg!("EXCEPTION: PAGE FAULT");
    dbg!("Accessed Address: {:?}", Cr2::read());
    dbg!("Error Code: {:?}", error_code);
    dbg!("{:#?}", stack_frame);
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

extern "x86-interrupt" fn general_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    dbg!("EXCEPTION: GENERAL PROTECTION FAULT");
    dbg!("{:?}", stack_frame);
    dbg!("{:b}", error_code);
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("{:?}", stack_frame);
    println!("{}", error_code);
    unsafe {
        asm!("hlt");
    }
}

/// Disable Programmable Interrupt Controller.
pub fn disable_pic() {
    // Set ICW1
    outb(0x20, 0x11);
    outb(0xa0, 0x11);

    // Set IWC2 (IRQ base offsets)
    outb(0x21, 0x20);
    outb(0xa1, 0x28);

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
