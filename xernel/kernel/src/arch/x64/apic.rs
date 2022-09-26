use core::arch::asm;

use crate::arch::x64::idt;
use libxernel::{boot::InitAtBoot, ticket::TicketMutex};
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

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
        dbg!("eoi");
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
        self.set_siv(0xFF);
        unsafe {
            self.write(0xF0, self.read(0xF0) | 1 << 8);
        }
    }

    /// Enable timer with a specific value.
    pub fn enable_timer(&mut self) {
        unsafe {
            self.write(0x3E0, 0x3);
            self.write(0x380, 0x10000);
            self.write(0x320, (1 << 17) | 0x40);
            dbg!("timer register is 0b{:b}", self.read(0x320));
        }
    }
}
