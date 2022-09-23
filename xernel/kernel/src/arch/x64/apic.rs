use core::arch::asm;

use libxernel::{boot::InitAtBoot, ticket::TicketMutex};
use x86_64::{PhysAddr, VirtAddr, structures::paging::PageTableFlags};

use crate::{acpi, println, print, mem::{HIGHER_HALF_OFFSET, vmm::KERNEL_PAGE_MAPPER}};

pub struct LocalAPIC {
    address: u64,
}

static APIC_BASE_ADDRESS: InitAtBoot<u64> = InitAtBoot::Uninitialized;

pub static APIC: TicketMutex<InitAtBoot<LocalAPIC>> = TicketMutex::new(InitAtBoot::Uninitialized);

pub fn init() {
    let apic_info = acpi::get_apic();
    println!("{:?}", apic_info);

    println!("{:x}", apic_info.local_apic_address);

    let apic_base = apic_info.local_apic_address + *HIGHER_HALF_OFFSET;

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    unsafe { mapper.map(PhysAddr::new(apic_info.local_apic_address), VirtAddr::new(apic_base), 
        PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE, true).unwrap();
    }

    let mut apic = APIC.lock();

    *apic = InitAtBoot::Initialized(LocalAPIC { address: apic_base });

    apic.enable_apic();

    unsafe {
        println!("{:x}", apic.read(0x20));
        println!("{:x}", apic.read(0x30));
        println!("{:x}", apic.read(0xF0));
        println!("{:x}", apic.read(0x320));

        asm!("sti");
    }
    

}

impl LocalAPIC {

    pub unsafe fn read(&self, reg: u64) -> u32 {
        ((self.address + reg) as *const u32).read_volatile()
    }

    pub unsafe fn write(&mut self, reg: u64, val: u32) {
        ((self.address + reg) as *mut u32).write_volatile(val);
    }

    pub fn eoi(&mut self) {

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
}