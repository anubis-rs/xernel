use crate::acpi::hpet;
use crate::arch::amd64::rdmsr;
use crate::debug;
use crate::mem::paging::KERNEL_PAGE_MAPPER;
use crate::mem::HIGHER_HALF_OFFSET;
use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

const IA32_APIC_BASE_MSR: u32 = 0x1B;
const IA32_TSC_DEADLINE_MSR: u32 = 0x6E0;

const LAPICRegID: u64 = 0x20;
const LAPICRegTPR: u64 = 0x80; // Task Priority Register
const LAPICRegEOI: u64 = 0xB0;
const LAPICRegSpurious: u64 = 0xF0;
const LAPICRegICR0: u64 = 0x300; // Interrupt Command Register
const LAPICRegICR1: u64 = 0x310;
const LAPICRegTimer: u64 = 0x320;
const LAPICRegTimerInitial: u64 = 0x380;
const LAPICRegTimerCurrentCount: u64 = 0x390;
const LAPICRegTimerDivider: u64 = 0x3e0;

pub struct LocalApic {
    address: u64,
    frequency: u64,
}

impl LocalApic {
    pub fn new() -> Self {
        let mut mapper = KERNEL_PAGE_MAPPER.lock();

        let mut apic_base = unsafe { rdmsr(IA32_APIC_BASE_MSR) };

        // INFO: IA32_APIC_BASE_MSR contains two flags on bit 8 and bit 11
        // BSP flag, bit 8 ⎯ Indicates if the processor is the bootstrap processor (BSP).
        // APIC Global Enable flag, bit 11 ⎯ Enables or disables the local APIC
        // To get the local apic base address, bit range 12 - 35, we set the flag bits to zero
        apic_base &= !(1 << 8);
        apic_base &= !(1 << 11);

        debug!("apic base: {:x}", apic_base);

        mapper.map::<Size4KiB>(
            PhysFrame::containing_address(PhysAddr::new(apic_base)),
            Page::containing_address(VirtAddr::new(apic_base + *HIGHER_HALF_OFFSET)),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
            true,
        );

        let mut lapic = LocalApic {
            address: apic_base + *HIGHER_HALF_OFFSET,
            frequency: 0,
        };

        lapic.enable_apic();
        lapic.init_timer_frequency();

        lapic
    }

    pub unsafe fn read(&self, reg: u64) -> u32 {
        ((self.address + reg) as *const u32).read_volatile()
    }

    pub unsafe fn write(&self, reg: u64, val: u32) {
        ((self.address + reg) as *mut u32).write_volatile(val);
    }

    pub fn lapic_id(&self) -> u32 {
        unsafe { self.read(LAPICRegID) }
    }

    pub fn eoi(&self) {
        unsafe { self.write(LAPICRegEOI, 0) }
    }

    // Spurious interrupt vector
    pub fn siv(&self) -> u32 {
        unsafe { self.read(LAPICRegSpurious) }
    }

    pub fn set_siv(&self, value: u32) {
        unsafe { self.write(LAPICRegSpurious, value) }
    }

    pub fn enable_apic(&self) {
        unsafe {
            self.set_siv(0x1ff);

            // set the task priority to 0
            self.write(LAPICRegTPR, 0);
        }
    }

    pub fn periodic_timer(&self, int_no: u8, micro_seconds_period: u64) {
        let mut apic_ticks = self.frequency * micro_seconds_period / (1000 * 1000);
        apic_ticks /= 16;

        unsafe {
            // set divider to 16
            self.write(LAPICRegTimerDivider, 3);

            // set the interrupt vector & oneshot mode
            self.write(LAPICRegTimer, (1 << 17) | int_no as u32);

            // set the counter to the calculated value
            self.write(LAPICRegTimerInitial, apic_ticks as u32);
        }
    }

    pub fn oneshot(&self, int_no: u8, micro_seconds: u64) {
        let mut apic_ticks = self.frequency * micro_seconds / (1000 * 1000);
        apic_ticks /= 16;

        unsafe {
            // set divider to 16
            self.write(LAPICRegTimerDivider, 3);

            // set the interrupt vector & periodic mode
            self.write(LAPICRegTimer, int_no as u32);

            // set the counter to the calculated value
            self.write(LAPICRegTimerInitial, apic_ticks as u32);
        }
    }

    pub fn deadline(&self, int_no: u8, nano_seconds: u64) {
        unsafe {
            // set the interrupt vector & deadline mode
            self.write(LAPICRegTimer, (2 << 17) | int_no as u32);

            // https://xem.github.io/minix86/manual/intel-x86-and-64-manual-vol3/o_fe12b1e2a880e0ce-379.html
            // IA32_TSC_DEADLINE_MSR
        }
    }

    pub fn stop(&self) {
        unsafe {
            self.write(LAPICRegTimerInitial, 0);
        }
    }

    pub fn send_ipi(&self, lapic_id: u32, vec: u32) {
        unsafe {
            self.write(LAPICRegICR1, lapic_id << 24);
            self.write(LAPICRegICR0, vec);
        }
    }

    pub fn init_timer_frequency(&mut self) {
        unsafe {
            // set the divisor to 1
            self.write(LAPICRegTimerDivider, 0b1011);

            let hpet_cycles_to_wait = hpet::frequency() / 100;

            let hpet_start_counter = hpet::read_main_counter();

            // set the initial count to 0xffffffff
            self.write(LAPICRegTimerInitial, 0xffffffff);

            // wait for 10 ms
            while hpet::read_main_counter() - hpet_start_counter < hpet_cycles_to_wait {}

            let apic_ticks = 0xffffffff - self.read(LAPICRegTimerCurrentCount);

            let hpet_end_counter = hpet::read_main_counter();

            let hpet_ticks = hpet_end_counter - hpet_start_counter;

            let apic_frequency = apic_ticks as u64 * hpet::frequency() / hpet_ticks;

            self.frequency = apic_frequency;
        }
    }
}
