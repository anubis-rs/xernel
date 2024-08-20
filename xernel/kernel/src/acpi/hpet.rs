use core::hint::spin_loop;

use libxernel::sync::Once;
use x86_64::{
    structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::mem::{paging::KERNEL_PAGE_MAPPER, HIGHER_HALF_OFFSET};

use super::ACPI;

const HPET_CONFIGURATION_REGISTER_OFFSET: u64 = 0x10;
const HPET_MAIN_COUNTER_REGISTER_OFFSET: u64 = 0xF0;

static HPET_FREQUENCY: Once<u64> = Once::new();
static HPET_PERIOD: Once<u64> = Once::new();
static HPET_CLOCK_TICK_UNIT: Once<u16> = Once::new();
static HPET_BASE_ADDRESS: Once<u64> = Once::new();

pub fn init() {
    let hpet_info = acpi_parsing::HpetInfo::new(&ACPI.tables).unwrap();

    HPET_CLOCK_TICK_UNIT.set_once(hpet_info.clock_tick_unit);
    HPET_BASE_ADDRESS.set_once(hpet_info.base_address as u64 + *HIGHER_HALF_OFFSET);

    let mut mapper = KERNEL_PAGE_MAPPER.lock();

    mapper.map::<Size4KiB>(
        PhysFrame::containing_address(PhysAddr::new(hpet_info.base_address as u64)),
        Page::containing_address(VirtAddr::new(hpet_info.base_address as u64 + *HIGHER_HALF_OFFSET)),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        true,
    );

    let period = (read(0) >> 32) & u64::MAX;

    assert!(period != 0);

    // INFO: Period must be less or equal to 0x05F5E100 according to HPET spec
    assert!(period <= 0x05F5E100);

    let f = (u64::pow(10, 15) as f64 / period as f64) as u64;

    debug!("HPET Period: {} femtoseconds", period);
    debug!("HPET Period: {} nanoseconds", period / 1_000_000);
    debug!("HPET Frequency: {} Hz", f);
    debug!("HPET Frequency: {} MHz", f / 1_000_000);

    HPET_FREQUENCY.set_once(f);
    HPET_PERIOD.set_once(period);

    // set ENABLE_CNF bit
    write(
        HPET_CONFIGURATION_REGISTER_OFFSET,
        read(HPET_CONFIGURATION_REGISTER_OFFSET) | 1,
    );
}

pub fn read_main_counter() -> u64 {
    read(HPET_MAIN_COUNTER_REGISTER_OFFSET)
}

pub fn frequency() -> u64 {
    *HPET_FREQUENCY
}

pub fn sleep(nanos: u64) {
    let target_counter = read_main_counter() + ((nanos * 1_000_000) / *HPET_PERIOD);

    while read_main_counter() < target_counter {
        spin_loop();
    }
}

/// returns the number of microseconds since start of the hpet
pub fn microseconds() -> u64 {
    read_main_counter() / (frequency() / 1_000_000)
}

/// returns the number of milliseconds since start of the hpet
pub fn milliseconds() -> u64 {
    read_main_counter() / (frequency() / 1_000)
}

fn write(offset: u64, val: u64) {
    let hpet_ptr = *HPET_BASE_ADDRESS as *mut u64;

    unsafe {
        core::ptr::write_volatile(hpet_ptr.byte_offset(offset as isize), val);
    }
}

fn read(offset: u64) -> u64 {
    let hpet_ptr = *HPET_BASE_ADDRESS as *mut u64;

    unsafe { core::ptr::read_volatile(hpet_ptr.byte_offset(offset as isize) as *const u64) }
}
