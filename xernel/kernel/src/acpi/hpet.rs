use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::{
    mem::{vmm::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET},
    print, println,
};

use super::ACPI;

pub fn init() {
    let hpet_info = acpi_parsing::HpetInfo::new(&ACPI.tables).unwrap();

    println!("{:?}", hpet_info);

    println!("{:x}", hpet_info.base_address);

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    unsafe {
        mapper.map(
            PhysAddr::new(hpet_info.base_address as u64),
            VirtAddr::new(hpet_info.base_address as u64 + *HIGHER_HALF_OFFSET),
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
            true,
        );

        let hpet_ptr = (*HIGHER_HALF_OFFSET + hpet_info.base_address as u64) as *const u64;

        let period = (hpet_ptr.read_volatile() >> 32) & u64::MAX;

        println!("0x{:x}", period);

        let f = u64::pow(10, 15) / period;

        println!("0x{:x}", f);

        println!("{:x}", (core::ptr::read_volatile(hpet_ptr) >> 8) & 0b1111);
    }
}
