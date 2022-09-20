use core::ptr::NonNull;

use crate::mem::HIGHER_HALF_OFFSET;
use crate::{print, println};
use acpi_parsing::platform::interrupt::Apic;
use acpi_parsing::{AcpiHandler, AcpiTables, PhysicalMapping};
use limine::LimineRsdpRequest;

#[derive(Clone)]
struct OffsetAcpiHandler;

static RSDP_REQUEST: LimineRsdpRequest = LimineRsdpRequest::new(0);

lazy_static! {
    pub static ref ACPI: Acpi = Acpi::new();
}

pub fn init() {
    lazy_static::initialize(&ACPI);

    println!("{:?}", ACPI.apic_info);
}

impl AcpiHandler for OffsetAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        PhysicalMapping::new(
            physical_address,
            NonNull::new_unchecked((physical_address + *HIGHER_HALF_OFFSET as usize) as *mut _),
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // the region is never unmapped
    }
}

#[derive(Debug)]
pub struct Acpi {
    pub apic_info: Apic,
}

impl Acpi {
    fn new() -> Self {
        let address = RSDP_REQUEST.get_response().get().unwrap().address.as_ptr();

        let acpi_tables = unsafe {
            AcpiTables::from_rsdp(
                OffsetAcpiHandler,
                address.unwrap() as usize - *HIGHER_HALF_OFFSET as usize,
            )
            .expect("failed to get acpi tables")
        };

        let plat_info = acpi_tables
            .platform_info()
            .expect("failed to get platform info");

        let apic_info = match plat_info.interrupt_model {
            acpi_parsing::InterruptModel::Apic(apic_info) => apic_info,
            _ => panic!("no apic in this system"),
        };

        Self { apic_info }
    }
}
