use core::arch::asm;

use libxernel::{boot::InitAtBoot, ticket::TicketMutex};
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::acpi::hpet;
use crate::{
    acpi, dbg,
    mem::{vmm::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET},
    print, println,
};

pub struct LocalAPIC {
    address: u64,
}

pub static APIC: TicketMutex<InitAtBoot<LocalAPIC>> = TicketMutex::new(InitAtBoot::new());

pub fn init() {
    let apic_info = acpi::get_apic();
    println!("{:?}", apic_info);

    println!("{:x}", apic_info.local_apic_address);

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

    apic.set_once(LocalAPIC { address: apic_base });

    apic.enable_apic();

    // calculate frequency of APIC timer
    unsafe {
        // set the divisor to 1
        apic.write(0x3e0, 1);

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

        println!("hpet_ticks: {}, apic_ticks: {}", hpet_ticks, apic_ticks);
        println!("APIC frequency: {} Hz", apic_frequency);
        println!("HPET frequency: {} Hz", hpet::frequency());
    }

    unsafe {
        println!("{:x}", apic.read(0x20));
        println!("{:x}", apic.read(0x30));
        println!("{:x}", apic.read(0xF0));
        println!("{:x}", apic.read(0x320));

        asm!("sti");
    }

    apic.enable_timer();
}

pub extern "x86-interrupt" fn timer(stack_frame: InterruptStackFrame) {
    let mut apic = APIC.lock();

    dbg!("timer");

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
            self.set_siv(self.read(0xF0) | 1 << 8);
        }
    }

    /// Enable timer with a specific value.
    pub fn enable_timer(&mut self) {
        unsafe {
            self.write(0x3E0, 3);
            self.write(0x380, 0xfffffff);
            self.write(0x320, (1 << 17) | 0x40);
            dbg!("timer register is 0b{:b}", self.read(0x320));
        }
    }
}
