use crate::arch::amd64::ports::outb;
use crate::sched::context::ThreadContext;
use core::arch::asm;
use core::mem::size_of;
use libxernel::sync::{Spinlock, SpinlockIRQ};

use paste::paste;
use seq_macro::seq;

const IDT_ENTRIES: usize = 256;

macro_rules! has_error_code_macro {
    (true) => {
        "nop"
    };
    (false) => {
        "push 0"
    };
}

macro_rules! interrupt_handler {
    ($interrupt_number:expr, $has_error_code:expr) => {
        paste! {
            #[naked]
            extern "C" fn [<interrupt_handler $interrupt_number>]() {
                unsafe {
                    asm!(
                        has_error_code_macro!($has_error_code),
                        "push r15",
                        "push r14",
                        "push r13",
                        "push r12",
                        "push r11",
                        "push r10",
                        "push r9",
                        "push r8",
                        "push rdi",
                        "push rsi",
                        "push rdx",
                        "push rcx",
                        "push rbx",
                        "push rax",
                        "push rbp",
                        "push rax",
                        concat!("mov rdi, ", $interrupt_number),
                        "mov rsi, rsp",
                        "call generic_interrupt_handler",
                        "add rsp, 0x8",
                        "mov rsp, rdi",
                        "pop rbp",
                        "pop rax",
                        "pop rbx",
                        "pop rcx",
                        "pop rdx",
                        "pop rsi",
                        "pop rdi",
                        "pop r8",
                        "pop r9",
                        "pop r10",
                        "pop r11",
                        "pop r12",
                        "pop r13",
                        "pop r14",
                        "pop r15",
                        "iretq",
                        options(noreturn)
                    )
                }
            }
        }
    };
}

seq!(N in 0..=7 { interrupt_handler!(N, false); });

interrupt_handler!(8, true);
interrupt_handler!(9, false);

seq!(N in 10..=14 { interrupt_handler!(N, true); });

interrupt_handler!(15, false);
interrupt_handler!(16, true);
interrupt_handler!(17, true);
interrupt_handler!(18, false);
interrupt_handler!(19, false);
interrupt_handler!(20, false);
interrupt_handler!(21, true);
interrupt_handler!(22, false);
interrupt_handler!(23, false);
interrupt_handler!(24, false);
interrupt_handler!(25, false);
interrupt_handler!(26, false);
interrupt_handler!(27, false);
interrupt_handler!(28, false);
interrupt_handler!(29, true);
interrupt_handler!(30, true);

seq!(N in 31..256 { interrupt_handler!(N, false); });

#[repr(C, packed)]
struct Idtr {
    size: u16,
    offset: u64,
}

impl Idtr {
    #[inline]
    const fn new(size: u16, offset: u64) -> Self {
        Self { size, offset }
    }

    #[inline(always)]
    unsafe fn load(&self) {
        asm!("lidt [{}]", in(reg) self, options(nostack));
    }
}

#[derive(Copy, Clone)]
pub(super) enum IRQHandler {
    ErrorHandler(fn(u8, ThreadContext)),
    Handler(fn(ThreadContext)),
    None,
}

static INTERRUPT_HANDLERS: SpinlockIRQ<[IRQHandler; IDT_ENTRIES]> =
    SpinlockIRQ::new([IRQHandler::None; IDT_ENTRIES]);

#[repr(packed)]
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

    pub(crate) fn set_handler(&mut self, handler: *const u8) {
        self.offset_low = handler as u16;
        self.offset_mid = (handler as usize >> 16) as u16;
        self.offset_hi = (handler as usize >> 32) as u32;
        self.flags = 0x8e;
        self.selector = 8;
    }
}

static mut IDT: [IDTEntry; IDT_ENTRIES] = [IDTEntry::NULL; IDT_ENTRIES];

pub fn init() {
    unsafe {
        seq!(N in 0..256 {
                #(
                    IDT[N].set_handler(interrupt_handler~N as *const u8);
                )*
        });

        let idtr = Idtr::new(
            ((IDT.len() * size_of::<IDTEntry>()) - 1) as u16,
            (&IDT as *const _) as u64,
        );

        idtr.load();
    }
}

#[no_mangle]
extern "C" fn generic_interrupt_handler(isr: usize, error_code: u8, ctx: ThreadContext) {
    let handlers = INTERRUPT_HANDLERS.lock();

    match &handlers[isr] {
        IRQHandler::Handler(handler) => {
            let handler = *handler;
            handlers.unlock();
            handler(ctx);
        }

        IRQHandler::ErrorHandler(handler) => {
            let handler = *handler;
            handlers.unlock();
            handler(error_code, ctx);
        }

        IRQHandler::None => panic!("unhandled interrupt {}", isr),
    }
}

pub fn allocate_vector() -> u8 {
    static FREE_VECTOR: Spinlock<u8> = Spinlock::new(32);

    let mut free_vector = FREE_VECTOR.lock();

    if *free_vector == 0xf0 {
        panic!("IDT exhausted");
    }

    let ret = *free_vector;

    *free_vector += 1;

    return ret;
}

pub fn register_handler(vector: u8, handler: fn(ThreadContext)) {
    let mut handlers = INTERRUPT_HANDLERS.lock();

    match handlers[vector as usize] {
        IRQHandler::None => {}
        _ => unreachable!("register_handler: handler has already been registered"),
    }

    handlers[vector as usize] = IRQHandler::Handler(handler);
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
