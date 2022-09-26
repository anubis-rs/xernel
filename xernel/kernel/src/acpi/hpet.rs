use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};
use libxernel::boot::InitAtBoot;

use crate::{
    mem::{vmm::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET},
    print, println, dbg,
};

use super::ACPI;

const HPET_CONFIGURATION_REGISTER_OFFSET: u64 = 0x10;
const HPET_MAIN_COUNTER_REGISTER_OFFSET: u64 = 0xF0;

static HPET_FREQUENCY: InitAtBoot<u64> = InitAtBoot::new();
static HPET_CLOCK_TICK_UNIT: InitAtBoot<u16> = InitAtBoot::new();
static HPET_BASE_ADDRESS: InitAtBoot<u64> = InitAtBoot::new();

pub fn init() {
    let hpet_info = acpi_parsing::HpetInfo::new(&ACPI.tables).unwrap();

    HPET_CLOCK_TICK_UNIT.set_once(hpet_info.clock_tick_unit);
    HPET_BASE_ADDRESS.set_once(hpet_info.base_address as u64 + *HIGHER_HALF_OFFSET);

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    unsafe {
        mapper.map(
            PhysAddr::new(hpet_info.base_address as u64),
            VirtAddr::new(hpet_info.base_address as u64 + *HIGHER_HALF_OFFSET),
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
            true,
        ).unwrap();

        let period = (read(0) >> 32) & u64::MAX;
        let f = (u64::pow(10, 15) as f64 / period as f64) as u64;

        HPET_FREQUENCY.set_once(f);

        // set ENABLE_CNF bit
        write(HPET_CONFIGURATION_REGISTER_OFFSET, read(HPET_CONFIGURATION_REGISTER_OFFSET) | 1);
    }
}

pub fn read_main_counter() -> u64 {
    return read(HPET_MAIN_COUNTER_REGISTER_OFFSET);
}

pub fn frequency() -> u64 {
    return *HPET_FREQUENCY;
}

fn write(offset: u64, val: u64) {
    let hpet_ptr = *HPET_BASE_ADDRESS as *mut u64;

    unsafe {
        core::ptr::write_volatile(hpet_ptr.byte_offset(offset as isize) as *mut u64, val);
    }
}

fn read(offset: u64) -> u64 {
    let hpet_ptr = *HPET_BASE_ADDRESS as *mut u64;

    unsafe {
        core::ptr::read_volatile(hpet_ptr.byte_offset(offset as isize) as *const u64)
    }
}
