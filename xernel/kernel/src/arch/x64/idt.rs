use crate::arch::x64::ports::outb;
use crate::sched::context::ThreadContext;
use core::arch::asm;
use libxernel::sync::Spinlock;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;

use paste::paste;
use seq_macro::seq;

use crate::backtrace;

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

#[derive(Copy, Clone)]
pub(super) enum Handler {
    ErrorHandler(fn(u8, &mut ThreadContext)),
    Handler(fn(&mut ThreadContext)),
    None,
}

static INTERRUPT_HANDLERS: Spinlock<[Handler; IDT_ENTRIES]> =
    Spinlock::new([Handler::None; IDT_ENTRIES]);

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

static mut IDT: [IDTEntry; IDT_ENTRIES] = [IDTEntry::NULL; IDT_ENTRIES];

pub fn init() {
    unsafe {
        seq!(N in 0..256 {
                #(
                    IDT[N].set_handler(interrupt_handler~N as *const u8);
                )*
        });
    }

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

#[no_mangle]
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
