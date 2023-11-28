pub mod ipl;
pub mod dpc;
pub mod idt;
pub mod dpc_queue;

use crate::arch::amd64::{ports::outb, read_cr2};
use crate::sched::context::TrapFrame;
use core::arch::asm;
use core::sync::atomic::{compiler_fence, Ordering};
use idt::{IRQHandler, IDT_ENTRIES};
use ipl::IPL;

use self::dpc::{dpc_interrupt_dispatch, DPC_VECTOR};
use self::ipl::{get_spl, raise_spl, set_ipl};

use super::apic::apic_spurious_interrupt;
use libxernel::sync::{SpinlockIRQ, Spinlock};

static INTERRUPT_HANDLERS: SpinlockIRQ<[IRQHandler; IDT_ENTRIES]> = SpinlockIRQ::new([IRQHandler::None; IDT_ENTRIES]);

pub fn init() {
    idt::init();

    let mut handlers = INTERRUPT_HANDLERS.lock();

    handlers[0xD] = IRQHandler::Handler(general_fault_handler);
    handlers[0xE] = IRQHandler::Handler(page_fault_handler);
    handlers[0x8] = IRQHandler::Handler(double_fault_handler);
    handlers[0xF0] = IRQHandler::Handler(apic_spurious_interrupt);

    let dpc_vector = allocate_vector(IPL::IPLDPC).expect("Could not allocate DPC Vector");

    DPC_VECTOR.set_once(dpc_vector);

    info!("DPC_VECTOR set to: {}", *DPC_VECTOR);

    handlers[dpc_vector as usize] = IRQHandler::Handler(dpc_interrupt_dispatch);
}

#[no_mangle]
extern "sysv64" fn generic_interrupt_handler(isr: usize, ctx: *mut TrapFrame) {

    let mut ipl = IPL::from(isr / 16);

    if (ipl as u8) < (get_spl() as u8) {
        println!("IPL not less or equal (running at {:?}, requested ipl {:?})", get_spl(), ipl);
        panic!("IPL not less or equal");
    }

    debug!("IRQL {:?} received while running on {:?}", ipl, get_spl());

    ipl = raise_spl(ipl);

    let handlers = INTERRUPT_HANDLERS.lock();

    let ctx = unsafe { &mut *ctx };

    match &handlers[isr] {
        IRQHandler::Handler(handler) => {
            enable();
            let handler = *handler;
            handlers.unlock();
            handler(ctx);
        }

        IRQHandler::None => panic!("unhandled interrupt {}", isr),
    }

    set_ipl(ipl);
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

// pub fn allocate_vector() -> u8 {
//     static FREE_VECTOR: Spinlock<u8> = Spinlock::new(0x20);

//     let mut free_vector = FREE_VECTOR.lock();

//     if *free_vector >= 0xf0 {
//         panic!("IDT exhausted");
//     }

//     let ret = *free_vector;

//     *free_vector += 1;

//     ret
// }

pub fn allocate_vector(ipl: IPL) -> Option<u8> {
    static FREE_VECTORS_FOR_IPL: Spinlock<[u8; 16]> = Spinlock::new([
        0x0 << 4,
        0x1 << 4,
        0x2 << 4,
        0x3 << 4,
        0x4 << 4,
        0x5 << 4,
        0x6 << 4,
        0x7 << 4,
        0x8 << 4,
        0x9 << 4,
        0xA << 4, 
        0xB << 4,
        0xC << 4,
        0xD << 4,
        0xE << 4,
        0xF << 4
    ]);

    if (ipl as u8) > 15 {
        return None;
    }

    let base_vector = (ipl as u8) << 4;

    let mut free_vectors = FREE_VECTORS_FOR_IPL.lock();

    let next_free_vector = free_vectors[ipl as usize];

    if next_free_vector > base_vector + 15 {
        return None;
    }

    free_vectors[ipl as usize] += 1;

    Some(next_free_vector)
}

pub fn register_handler(vector: u8, handler: fn(&mut TrapFrame)) {
    let mut handlers = INTERRUPT_HANDLERS.lock();

    match handlers[vector as usize] {
        IRQHandler::None => {}
        _ => unreachable!("register_handler: handler has already been registered"),
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

