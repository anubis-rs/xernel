use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB};
use crate::acpi::hpet;
use crate::arch::amd64::rdmsr;
use crate::mem::HIGHER_HALF_OFFSET;
use crate::mem::paging::KERNEL_PAGE_MAPPER;

const IA32_APIC_BASE_MSR: u32 = 0x1B;
const IA32_TSC_DEADLINE_MSR: u32 = 0x6E0;

pub struct LocalApic {
    address: u64,
    frequency: u64,
}

impl LocalApic {
    pub fn new() -> Self {
        let mut mapper = KERNEL_PAGE_MAPPER.lock();
        
        let mut apic_base = unsafe {
            rdmsr(IA32_APIC_BASE_MSR)
        };

        // INFO: IA32_APIC_BASE_MSR contains to flags on bit 8 and bit 11
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
        unsafe { self.read(0x20) }
    }

    pub fn eoi(&self) {
        unsafe { self.write(0xB0, 0) }
    }

    // Spurious interrupt vector
    pub fn siv(&self) -> u32 {
        unsafe { self.read(0xF0) }
    }

    pub fn set_siv(&self, value: u32) {
        unsafe { self.write(0xF0, value) }
    }

    pub fn enable_apic(&self) {
        unsafe {
            self.set_siv(0x1ff);

            // set the task priority to 0
            self.write(0x80, 0);
        }
    }

    pub fn periodic_timer(&self, int_no: u8, micro_seconds_period: u64) {
        let mut apic_ticks = self.frequency * micro_seconds_period / (1000 * 1000);
        apic_ticks /= 16;

        unsafe {
            // set divider to 16
            self.write(0x3e0, 3);

            // set the interrupt vector & oneshot mode
            self.write(0x320, (1 << 17) | int_no as u32);

            // set the counter to the calculated value
            self.write(0x380, apic_ticks as u32);
        }
    }

    pub fn oneshot(&self, int_no: u8, micro_seconds: u64) {
        let mut apic_ticks = self.frequency * micro_seconds / (1000 * 1000);
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

    pub fn deadline(&self, int_no: u8, nano_seconds: u64) {
        unsafe {
            // set the interrupt vector & deadline mode
            self.write(0x320, (2 << 17) | int_no as u32);

            // https://xem.github.io/minix86/manual/intel-x86-and-64-manual-vol3/o_fe12b1e2a880e0ce-379.html
            // IA32_TSC_DEADLINE_MSR
        }
    }

    pub fn stop(&self) {
        unsafe {
            self.write(0x380, 0);
        }
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
}
