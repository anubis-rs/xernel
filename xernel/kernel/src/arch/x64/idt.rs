use crate::arch::x64::apic::apic_spurious_interrupt;
use crate::arch::x64::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::arch::x64::ports::outb;
use crate::drivers::ps2::keyboard::keyboard;
use crate::sched::context::ThreadContext;
use crate::sched::scheduler::scheduler_irq_handler;
use core::arch::{asm, global_asm};
use libxernel::boot::InitAtBoot;
use libxernel::sync::Spinlock;
use x86_64::instructions::tables::lidt;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::idt::{InterruptStackFrame};
use x86_64::{set_general_handler, VirtAddr};

use crate::backtrace;

const IDT_ENTRIES: usize = 256;

global_asm!(include_str!("int_thunks.S"));

#[derive(Copy, Clone)]
pub(super) enum Handler {
    ErrorHandler(fn(u8, &mut ThreadContext)),
    Handler(fn(&mut ThreadContext)),

    None,
}

static INTERRUPT_HANDLERS: Spinlock<[Handler; IDT_ENTRIES]> = Spinlock::new([Handler::None; IDT_ENTRIES]);

#[repr(C, packed)]
pub struct IDTEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_hi: u32,
    reserved: u32,
}

impl IDTEntry {
    const NULL: Self = Self {
        offset_low: 0x00,
        selector: 0x00,
        ist: 0x00,
        flags: 0x00,
        offset_mid: 0x00,
        offset_hi: 0x00,
        reserved: 0x00,
    };

    fn set_offset(&mut self, selector: u16, base: usize) {
        self.selector = selector;
        self.offset_low = base as u16;
        self.offset_mid = (base >> 16) as u16;
        self.offset_hi = (base >> 32) as u32;
    }

    /// Set the handler function of the IDT entry.
    pub(crate) fn set_handler(&mut self, handler: *const u8) {
        self.set_offset(8, handler as usize);
        self.flags = 0x8e;
    }
}

#[repr(C, packed)]
pub struct IDT {
    entries: [IDTEntry; IDT_ENTRIES]
}

static mut IDT: [IDTEntry; IDT_ENTRIES] = [IDTEntry::NULL; IDT_ENTRIES];

//pub static mut IDT: InitAtBoot<InterruptDescriptorTable> = InitAtBoot::Uninitialized;

pub fn init() {
    // let mut idt = IDT::new();

    // set_general_handler!(&mut idt, interrupt_handler);
    // unsafe {
    //     idt.double_fault
    //         .set_handler_fn(double_fault_handler)
    //         .set_stack_index(DOUBLE_FAULT_IST_INDEX);
    // }
    // idt.page_fault.set_handler_fn(page_fault_handler);
    // idt.general_protection_fault
    //     .set_handler_fn(general_fault_handler);

    // unsafe {
    //     idt[0x40].set_handler_addr(VirtAddr::new(scheduler_irq_handler as u64));
    // }
    // idt[0x47].set_handler_fn(keyboard);
    // idt[0xff].set_handler_fn(apic_spurious_interrupt);

    // unsafe {
    //     IDT = InitAtBoot::Initialized(idt);
    //     IDT.load();
    // }

    extern "C" {
        // defined in `handlers.asm`
        static interrupt_handlers: [*const u8; IDT_ENTRIES];
    }

    unsafe {
        for (index, &handler) in interrupt_handlers.iter().enumerate() {
            // skip handler insertion if handler is null.
            if handler.is_null() {
                continue;
            }

            IDT[index].set_handler(handler);
        }
    }


}

extern "C" fn generic_interrupt_handler(isr: usize, ctx: &mut ThreadContext) {
    let handlers = INTERRUPT_HANDLERS.lock();

    match &handlers[isr] {
        Handler::Handler(handler) => {
            let handler = *handler;
            core::mem::drop(handlers);
            handler(ctx);
        }

        Handler::ErrorHandler(handler) => {
            let handler = *handler;
            core::mem::drop(handlers);
            handler(0, ctx);
        }

        Handler::None => warning!("unhandled interrupt {}", isr),
    }
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
