use core::arch::asm;
use libxernel::once::Once;
use libxernel::ticket::TicketMutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::acpi::hpet;
use crate::{
    acpi, debug,
    mem::{vmm::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET},
    println,
};

pub struct LocalAPIC {
    address: u64,
}

pub static APIC: TicketMutex<LocalAPIC> = TicketMutex::new(LocalAPIC { address: 0 });
static APIC_FREQUENCY: Once<u64> = Once::new();

pub fn init() {
    let apic_info = acpi::get_apic();

    let apic_base = apic_info.local_apic_address + *HIGHER_HALF_OFFSET;

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    unsafe {
        mapper
            .map(
                PhysAddr::new(apic_info.local_apic_address),
                VirtAddr::new(apic_base),
                PageTableFlags::PRESENT
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::WRITABLE,
                true,
            )
            .unwrap();
    }

    let mut apic = APIC.lock();

    apic.address = apic_base;

    apic.enable_apic();

    // calculate frequency of APIC timer
    unsafe {
        // set the divisor to 1
        apic.write(0x3e0, 0b1011);

        let hpet_cycles_to_wait = hpet::frequency() / 100;

        let hpet_start_counter = hpet::read_main_counter();

        // set the initial count to 0xffffffff
        apic.write(0x380, 0xffffffff);

        // wait for 10 ms
        while hpet::read_main_counter() - hpet_start_counter < hpet_cycles_to_wait {}

        let apic_ticks = 0xffffffff - apic.read(0x390);

        let hpet_end_counter = hpet::read_main_counter();

        let hpet_ticks = hpet_end_counter - hpet_start_counter;

        let apic_frequency = apic_ticks as u64 * hpet::frequency() / hpet_ticks;

        //APIC_FREQUENCY = InitAtBoot::Initialized(apic_frequency);
        APIC_FREQUENCY.set_once(apic_frequency);
    }

    apic.create_periodic_timer(0x40, 1000 * 1000);
}

#[naked]
pub extern "C" fn timer(_stack_frame: InterruptStackFrame) {
    unsafe {
        asm!(
            "push r15;
            push r14; 
            push r13;
            push r12;
            push r11;
            push r10;
            push r9;
            push r8;
            push rdi;
            push rsi;
            push rdx;
            push rcx;
            push rbx;
            push rax;
            push rbp;
            call schedule_handle",
            options(noreturn)
        );
    }
}

pub extern "x86-interrupt" fn apic_spurious_interrupt(_stack_frame: InterruptStackFrame) {
    let mut apic = APIC.lock();

    apic.eoi();
}

impl LocalAPIC {
    pub unsafe fn read(&self, reg: u64) -> u32 {
        ((self.address + reg) as *const u32).read_volatile()
    }

    pub unsafe fn write(&mut self, reg: u64, val: u32) {
        ((self.address + reg) as *mut u32).write_volatile(val);
    }

    pub fn eoi(&mut self) {
        unsafe { self.write(0xB0, 0) }
    }

    // Spurious interrupt vector.
    pub fn siv(&self) -> u32 {
        unsafe { self.read(0xF0) }
    }

    pub fn set_siv(&mut self, value: u32) {
        unsafe { self.write(0xF0, value) }
    }

    pub fn enable_apic(&mut self) {
        unsafe {
            self.set_siv(0x1ff);

            // set the task priority to 0
            self.write(0x80, 0);
        }
    }

    pub fn create_periodic_timer(&mut self, int_no: u8, micro_seconds_period: u64) {
        let mut apic_ticks = *APIC_FREQUENCY * micro_seconds_period / (1000 * 1000);
        apic_ticks /= 16;

        unsafe {
            // set divider to 16
            self.write(0x3e0, 3);

            // set the interrupt vector & periodic mode
            self.write(0x320, (1 << 17) | int_no as u32);

            // set the counter to the calculated value
            self.write(0x380, apic_ticks as u32);
        }
    }

    pub fn create_oneshot_timer(&mut self, int_no: u8, micro_seconds_period: u64) {
        let mut apic_ticks = *APIC_FREQUENCY * micro_seconds_period / (1000 * 1000);
        apic_ticks /= 16;

        unsafe {
            // set divider to 16
            self.write(0x3e0, 3);

            // set the interrupt vector & periodic mode
            self.write(0x320, int_no as u32);

            // set the counter to the calculated value
            self.write(0x380, apic_ticks as u32);
        }
    }

    pub fn stop(&mut self) {
        unsafe {
            self.write(0x380, 0);
        }
    }
}
