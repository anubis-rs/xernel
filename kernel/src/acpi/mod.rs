pub mod hpet;

use core::ptr::NonNull;

use crate::mem::HIGHER_HALF_OFFSET;
use acpi_parsing::platform::interrupt::Apic;
use acpi_parsing::{AcpiHandler, AcpiTables, InterruptModel, PhysicalMapping};
use libxernel::sync::Once;
use limine::RsdpRequest;

#[derive(Clone)]
struct AcpiMapper;

static RSDP_REQUEST: RsdpRequest = RsdpRequest::new(0);

pub static ACPI: Once<Acpi> = Once::new();

pub fn init() {
    ACPI.set_once(Acpi::new());
}

impl AcpiHandler for AcpiMapper {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
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

pub struct Acpi {
    tables: AcpiTables<AcpiMapper>,
}

impl Acpi {
    pub fn new() -> Self {
        let address = RSDP_REQUEST.get_response().get().unwrap().address.as_ptr();

        let acpi_tables = unsafe {
            AcpiTables::from_rsdp(AcpiMapper, address.unwrap() as usize - *HIGHER_HALF_OFFSET as usize)
                .expect("failed to get acpi tables")
        };

        Self { tables: acpi_tables }
    }

    pub fn get_apic(&self) -> Apic {
        match ACPI.tables.platform_info().unwrap().interrupt_model {
            InterruptModel::Apic(apic) => apic,
            InterruptModel::Unknown => panic!("No apic found"),
            _ => unreachable!(),
        }
    }
}
