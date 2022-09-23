use libxernel::{boot::InitAtBoot, ticket::TicketMutex};
use x86_64::{PhysAddr, VirtAddr, structures::paging::PageTableFlags};

use crate::{acpi, println, print, mem::{HIGHER_HALF_OFFSET, vmm::KERNEL_PAGE_MAPPER}};

pub struct LocalAPIC {
    address: u64,
}

static APIC_BASE_ADDRESS: InitAtBoot<u64> = InitAtBoot::Uninitialized;

pub static APIC: TicketMutex<InitAtBoot<LocalAPIC>> = TicketMutex::new(InitAtBoot::Uninitialized);

pub fn init() {
    let apic = acpi::get_apic();
    println!("{:?}", apic);

    let apic_base = apic.local_apic_address + *HIGHER_HALF_OFFSET;

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    unsafe { mapper.map(PhysAddr::new(apic.local_apic_address), VirtAddr::new(apic_base), 
        PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE, true);
    }

    *APIC.lock() = InitAtBoot::Initialized(LocalAPIC { address: apic_base });
    

}

impl LocalAPIC {

    pub fn read(&self, reg: u32) -> u32 {
        reg
    }

    pub fn write(&mut self, reg: u32, val: u32) {

    }

    pub fn eoi(&mut self) {

    }

}