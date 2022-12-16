use acpi_parsing::platform::interrupt::Apic;
use alloc::vec::Vec;
use core::arch::asm;
use libxernel::sync::Spinlock;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::acpi::{hpet, ACPI};
use crate::mem::{vmm::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET};
use crate::{dbg, debug};

pub struct LocalAPIC {
    address: u64,
    frequency: u64,
}

pub struct IOApic {
    id: u8,
    /// u64 since we add HIGHER_HALF_OFFSET which wouldn't fit in a u32
    address: u64,
    interrupt_base: u32,
}

pub static IOAPICS: Spinlock<Vec<IOApic>> = Spinlock::new(Vec::new());

pub static APIC: Spinlock<LocalAPIC> = Spinlock::new(LocalAPIC {
    address: 0,
    frequency: 0,
});

pub fn init() {
    let apic_info = ACPI.get_apic();

    let mut io_apics = IOAPICS.lock();

    for ioapic in apic_info.io_apics.iter() {
        io_apics.push(IOApic {
            id: ioapic.id,
            address: (ioapic.address as u64) + *HIGHER_HALF_OFFSET,
            interrupt_base: ioapic.global_system_interrupt_base,
        });
    }

    let mut lapic = APIC.lock();

    let ioapic = io_apics.first_mut().unwrap();

    lapic.init(&apic_info);
    ioapic.init(&apic_info);
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

impl LocalAPIC {
    pub fn init(&mut self, apic_info: &Apic) {
        let mut mapper = KERNEL_PAGE_MAPPER.lock();

        let apic_base = apic_info.local_apic_address + *HIGHER_HALF_OFFSET;

        debug!("apic base: {:x}", apic_base);

        mapper.map(
            PhysAddr::new(apic_info.local_apic_address),
            VirtAddr::new(apic_base),
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
            true,
        );

        self.address = apic_base;

        self.enable_apic();

        self.init_timer_frequency();

        self.create_periodic_timer(0x40, 1000 * 1000);
    }

    pub fn init_timer_frequency(&mut self) {
        unsafe {
            // set the divisor to 1
            self.write(0x3e0, 0b1011);

            let hpet_cycles_to_wait = hpet::frequency() / 100;

            let hpet_start_counter = hpet::read_main_counter();

            // set the initial count to 0xffffffff
            self.write(0x380, 0xffffffff);

            // wait for 10 ms
            while hpet::read_main_counter() - hpet_start_counter < hpet_cycles_to_wait {}

            let apic_ticks = 0xffffffff - self.read(0x390);

            let hpet_end_counter = hpet::read_main_counter();

            let hpet_ticks = hpet_end_counter - hpet_start_counter;

            let apic_frequency = apic_ticks as u64 * hpet::frequency() / hpet_ticks;

            self.frequency = apic_frequency;
        }
    }

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
        let mut apic_ticks = self.frequency * micro_seconds_period / (1000 * 1000);
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
        let mut apic_ticks = self.frequency * micro_seconds_period / (1000 * 1000);
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

impl IOApic {
    // in read/write function cast as u32, since IOApic registers are 32 Bit or 64 Bit register should be handled as two 32 Bit register

    pub unsafe fn read(&self, reg: u32) -> u32 {
        ((self.address) as *mut u32).write_volatile(reg);
        ((self.address + 0x10) as *const u32).read_volatile()
    }

    pub unsafe fn write(&mut self, reg: u32, val: u32) {
        ((self.address) as *mut u32).write_volatile(reg);
        ((self.address + 0x10) as *mut u32).write_volatile(val);
    }

    pub unsafe fn mask_irq(&mut self) {}

    pub unsafe fn unmask_irq(&mut self) {}

    pub unsafe fn write_irq(
        &mut self,
        irq_number: u8,
        irq_vector: u8,
        apic_id: u8,
        level_sensitive: bool,
        low_priority: bool,
    ) {
        let redirection_entry = (0x10 + irq_number * 2) as u32;

        if !(0x10..=0xFE).contains(&irq_vector) {
            dbg!("[IOAPIC] write_irq: bad irq_vector {}", irq_vector);
        }

        if apic_id > 15 {
            dbg!("[IOAPIC] write_irq: bad apic_id {}", apic_id);
        }

        let mut val: u32 = 0;

        val = irq_vector as _;

        if low_priority {
            val |= 1 << 8;
        }

        // level_senstive descripes if edge or level sensitive
        // true stands for level sensitive therefore setting the according bit
        if level_sensitive {
            val |= 1 << 15;
        }

        // redirection entry has to be accessed as two 32-bit registers
        // creating own write value for higher register, since only receiver apic id is set in the reg
        let destination_field: u32 = (apic_id as u32) << 24;

        self.write(redirection_entry, val);
        self.write(redirection_entry + 1, destination_field);
    }

    pub fn init(&mut self, apic_info: &Apic) {
        debug!("{:?}", apic_info.interrupt_source_overrides);
        debug!("{:?}", apic_info.io_apics);

        let mut mapper = KERNEL_PAGE_MAPPER.lock();
        mapper.map_range(
            PhysAddr::new(self.address - *HIGHER_HALF_OFFSET),
            VirtAddr::new(self.address),
            0x2000,
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
            true,
        );

        unsafe {
            // IRQ 1 (keyboard) gets handled by 0x47 entry in IDT by CPU with APIC_ID 0, set as low
            // priority
            self.write_irq(1, 0x47, 0, false, true);
            debug!("IOAPICID: {:b}", self.read(0));
            debug!("IOAPICVER: {:b}", self.read(1));
            debug!("IOAPICARB: {:b}", self.read(2));
        }
    }
}

pub extern "x86-interrupt" fn apic_spurious_interrupt(_stack_frame: InterruptStackFrame) {
    let mut apic = APIC.lock();

    apic.eoi();
}
