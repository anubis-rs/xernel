pub mod hpet;

use core::ptr::NonNull;

use crate::mem::HIGHER_HALF_OFFSET;
use crate::{panic, print, println};
use acpi_parsing::platform::interrupt::Apic;
use acpi_parsing::{AcpiHandler, AcpiTables, InterruptModel, PhysicalMapping};
use limine::LimineRsdpRequest;

#[derive(Clone)]
struct AcpiMapper;

static RSDP_REQUEST: LimineRsdpRequest = LimineRsdpRequest::new(0);

lazy_static! {
    pub static ref ACPI: Acpi = Acpi::new();
}

pub fn init() {
    lazy_static::initialize(&ACPI);
}

impl AcpiHandler for AcpiMapper {
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

pub struct Acpi {
    tables: AcpiTables<AcpiMapper>,
}

impl Acpi {
    fn new() -> Self {
        let address = RSDP_REQUEST.get_response().get().unwrap().address.as_ptr();

        let acpi_tables = unsafe {
            AcpiTables::from_rsdp(
                AcpiMapper,
                address.unwrap() as usize - *HIGHER_HALF_OFFSET as usize,
            )
            .expect("failed to get acpi tables")
        };

        Self {
            tables: acpi_tables,
        }
    }
}

pub fn get_apic() -> Apic {
    match ACPI.tables.platform_info().unwrap().interrupt_model {
        InterruptModel::Apic(apic) => apic,
        InterruptModel::Unknown => panic!("No apic found"),
        _ => unreachable!(),
    }
}
