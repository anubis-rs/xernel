pub mod idt;

use crate::arch::amd64::apic::APIC;
use crate::arch::amd64::{ports::outb, read_cr2};
use crate::dpc::dispatch_dpcs;
use crate::drivers::ps2::keyboard::keyboard_handler;
use crate::sched::context::TrapFrame;
use core::arch::asm;
use core::sync::atomic::{Ordering, compiler_fence};
use idt::{IDT_ENTRIES, IRQHandler};
use libxernel::ipl::{IPL, get_ipl, raise_ipl, splx};

use super::apic::apic_spurious_interrupt;
use libxernel::sync::SpinlockIRQ;

static INTERRUPT_HANDLERS: SpinlockIRQ<[IRQHandler; IDT_ENTRIES]> = SpinlockIRQ::new([IRQHandler::None; IDT_ENTRIES]);

pub fn init() {
    idt::init();

    let mut handlers = INTERRUPT_HANDLERS.lock();

    handlers[0xD] = IRQHandler::Handler(general_fault_handler);
    handlers[0xE] = IRQHandler::Handler(page_fault_handler);
    handlers[0x8] = IRQHandler::Handler(double_fault_handler);
    handlers[0xF0] = IRQHandler::Handler(apic_spurious_interrupt);
    // TODO: allocate vectors accordingly or manually set all known interrupt handlers here
    handlers[0x2f] = IRQHandler::Handler(dispatch_dpcs);
    handlers[0xd0] = IRQHandler::Handler(keyboard_handler);
}

#[unsafe(no_mangle)]
extern "sysv64" fn generic_interrupt_handler(isr: usize, ctx: *mut TrapFrame) {
    let new_ipl = IPL::from(isr >> 4);
    let current_ipl = get_ipl();

    if (new_ipl as u8) < (current_ipl as u8) {
        panic!("IPL not less or equal");
    }

    raise_ipl(new_ipl);
    enable();

    let handlers = INTERRUPT_HANDLERS.lock();

    let ctx = unsafe { &mut *ctx };

    match &handlers[isr] {
        IRQHandler::Handler(handler) => {
            let handler = *handler;
            handlers.unlock();
            handler(ctx);
        }

        IRQHandler::None => panic!("unhandled interrupt {}", isr),
    }

    if isr > 32 {
        APIC.eoi();
    }

    disable();

    splx(current_ipl);
}

#[inline]
pub fn enable() {
    compiler_fence(Ordering::Release);
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

#[inline]
pub fn disable() {
    compiler_fence(Ordering::Acquire);
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

pub fn allocate_vector(ipl: IPL) -> Option<u8> {
    let starting = core::cmp::max((ipl as u8) << 4, 32);

    let handlers = INTERRUPT_HANDLERS.lock();

    for i in starting..starting + 16 {
        if let IRQHandler::None = handlers[i as usize] {
            return Some(i);
        }
    }

    None
}

pub fn register_handler(vector: u8, handler: fn(&mut TrapFrame)) {
    let mut handlers = INTERRUPT_HANDLERS.lock();

    match handlers[vector as usize] {
        IRQHandler::None => {}
        _ => panic!("register_handler: handler has already been registered"),
    }

    handlers[vector as usize] = IRQHandler::Handler(handler);
}

fn double_fault_handler(frame: &mut TrapFrame) {
    dbg!("EXCEPTION: DOUBLE FAULT");
    dbg!("{:#?}", frame);
    dbg!("{}", frame.error_code);
    println!("EXCEPTION: DOUBLE FAULT");
    println!("{:#?}", frame);
    println!("{}", frame.error_code);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

fn page_fault_handler(frame: &mut TrapFrame) {
    dbg!("EXCEPTION: PAGE FAULT");
    dbg!("Accessed Address: {:?}", read_cr2());
    dbg!("Error Code: {:?}", frame.error_code);
    dbg!("{:#?}", frame);
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", read_cr2());
    println!("Error Code: {:?}", frame.error_code);
    println!("{:#?}", frame);
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

fn general_fault_handler(frame: &mut TrapFrame) {
    dbg!("EXCEPTION: GENERAL PROTECTION FAULT");
    dbg!("{:?}", frame);
    dbg!("{:b}", frame.error_code);
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("{:?}", frame);
    println!("{}", frame.error_code);
    unsafe {
        asm!("hlt");
    }
}

/// Disable Programmable Interrupt Controller.
pub fn disable_pic() {
    unsafe {
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
}
