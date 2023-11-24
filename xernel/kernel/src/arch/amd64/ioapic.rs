use acpi_parsing::platform::interrupt::Apic;
use alloc::vec::Vec;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{PhysAddr, VirtAddr};

use crate::acpi::ACPI;
use crate::mem::{paging::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET};
use crate::{dbg, debug};

pub struct IOApic {
    id: u8,
    address: u64,
    interrupt_base: u32,
}

impl IOApic {
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

        let mut val = irq_vector as _;

        if low_priority {
            val |= 1 << 8;
        }

        // level_sensitive describes if edge or level sensitive
        // true stands for level sensitive therefore setting the according bit
        if level_sensitive {
            val |= 1 << 15;
        }

        // redirection_entry has to be accessed as two 32-bit registers
        // creating own write value for higher reigster, since only receiver apic id is set in the
        // reg
        let destination_field: u32 = (apic_id as u32) << 24;

        self.write(redirection_entry, val);
        self.write(redirection_entry + 1, destination_field);
    }

    pub fn init(&mut self, apic_info: &Apic) {
        debug!("{:?}", apic_info.io_apics);

        let mut mapper = KERNEL_PAGE_MAPPER.lock();
        mapper.map_range(
            PhysAddr::new(self.address - *HIGHER_HALF_OFFSET),
            VirtAddr::new(self.address),
            0x2000,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
            true,
        );

        unsafe {
            self.write_irq(1, 0x47, 0, false, true);
            debug!("IOAPICID: {:b}", self.read(0));
            debug!("IOAPICVER: {:b}", self.read(1));
            debug!("IOAPICARB: {:b}", self.read(2));
        }
    }
}

pub fn init(io_apics: &mut Vec<IOApic>) {
    let apic_info = ACPI.get_apic();

    for ioapic in apic_info.io_apics.iter() {
        io_apics.push(IOApic {
            id: ioapic.id,
            address: (ioapic.address as u64) + *HIGHER_HALF_OFFSET,
            interrupt_base: ioapic.global_system_interrupt_base,
        });
    }

    let ioapic = io_apics.first_mut().unwrap();
    ioapic.init(&apic_info);
}
