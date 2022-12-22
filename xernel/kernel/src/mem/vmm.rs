use super::{pmm::FRAME_ALLOCATOR, HIGHER_HALF_OFFSET};
use crate::{
    debug,
    mem::{pmm::MEMORY_MAP, FRAME_SIZE, KERNEL_OFFSET},
};
use libxernel::boot::InitAtBoot;
use libxernel::sync::Spinlock;
use limine::LimineKernelAddressRequest;
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::Size4KiB,
};
use x86_64::{
    structures::paging::{PageTable, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

static KERNEL_ADDRESS_REQUEST: LimineKernelAddressRequest = LimineKernelAddressRequest::new(0);

pub static KERNEL_PAGE_MAPPER: Spinlock<InitAtBoot<Pagemap>> =
    Spinlock::new(InitAtBoot::Uninitialized);

#[derive(Debug)]
pub struct Pagemap {
    page_table: *mut PageTable,
}

extern "C" {
    static _kernel_end: u64;
}

// TODO: Use results for return values for methods
// Create Error enum for mem module

impl Pagemap {
    pub fn new(pt_frame: Option<PhysFrame>) -> Self {
        let pt_frame = pt_frame.unwrap_or_else(|| {
            let mut frame_allocator = FRAME_ALLOCATOR.lock();

            frame_allocator.allocate_frame().unwrap()
        });

        let pt_address = unsafe {
            let ptr = (pt_frame.start_address().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;
            *ptr = PageTable::new();
            (*ptr).zero();
            ptr
        };

        Self {
            page_table: pt_address,
        }
    }

    pub fn map(&mut self, phys: PhysAddr, virt: VirtAddr, flags: PageTableFlags, _flush_tlb: bool) {
        let pml4 = self.page_table;

        let mut frame_allocator = FRAME_ALLOCATOR.lock();

        unsafe {
            let pml4_entry = &mut (*pml4)[virt.p4_index()];

            if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml4_entry.set_addr(PhysAddr::new(address), flags);
            }

            let pml3 = (pml4_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml3_entry = &mut (*pml3)[virt.p3_index()];

            if !pml3_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml3_entry.set_addr(PhysAddr::new(address), flags);
            }

            let pml2 = (pml3_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml2_entry = &mut (*pml2)[virt.p2_index()];

            if !pml2_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml2_entry.set_addr(PhysAddr::new(address), flags);
            }

            let pml1 = (pml2_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml1_entry = &mut (*pml1)[virt.p1_index()];

            pml1_entry.set_addr(phys, flags);
        }
    }

    pub fn map_range(
        &mut self,
        phys: PhysAddr,
        virt: VirtAddr,
        amount: usize,
        flags: PageTableFlags,
        flush_tlb: bool,
    ) {
        let mut pages_to_map = amount as u64 / FRAME_SIZE;

        if amount as u64 % FRAME_SIZE != 0 {
            pages_to_map += 1;
        }

        for i in 0..pages_to_map {
            let phys_addr = PhysAddr::new(phys.as_u64() + i as u64 * FRAME_SIZE);
            let virt_addr = VirtAddr::new(virt.as_u64() + i as u64 * FRAME_SIZE);

            self.map(phys_addr, virt_addr, flags, flush_tlb);
        }
    }

    pub unsafe fn load_pt(&mut self) {
        let pt = self.page_table;
        let phys = pt as *const _ as u64 - *HIGHER_HALF_OFFSET;

        Cr3::write(
            PhysFrame::from_start_address(PhysAddr::new(phys)).unwrap(),
            Cr3Flags::empty(),
        );
    }

    pub fn map_kernel(&mut self) {
        debug!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

        let kernel_base_address = KERNEL_ADDRESS_REQUEST
            .get_response()
            .get()
            .unwrap()
            .physical_base;
        let kernel_virt_address = KERNEL_ADDRESS_REQUEST
            .get_response()
            .get()
            .unwrap()
            .virtual_base;

        debug!("Kernel Base Address: {:x}", kernel_base_address);
        debug!("Kernel Virt Address: {:x}", kernel_virt_address);
        unsafe {
            let kernel_size = ((&_kernel_end as *const u64) as u64) - kernel_virt_address;
            debug!(
                "Kernel Size: {:x}",
                ((&_kernel_end as *const u64) as u64) - kernel_virt_address
            );

            self.map_range(
                PhysAddr::new(kernel_base_address),
                VirtAddr::new(KERNEL_OFFSET),
                kernel_size as usize,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                false,
            )
        }
    }

    pub fn unmap(&mut self, virt: VirtAddr) {
        let pml4 = self.page_table;

        unsafe {
            if !(*pml4)[virt.p4_index()]
                .flags()
                .contains(PageTableFlags::PRESENT)
            {
                return;
            }

            let pml3 = (*pml4)[virt.p4_index()].addr().as_u64() as *mut PageTable;

            if !(*pml3)[virt.p3_index()]
                .flags()
                .contains(PageTableFlags::PRESENT)
            {
                return;
            }

            let pml2 = (*pml3)[virt.p3_index()].addr().as_u64() as *mut PageTable;

            if !(*pml2)[virt.p2_index()]
                .flags()
                .contains(PageTableFlags::PRESENT)
            {
                return;
            }

            let pml1 = (*pml2)[virt.p2_index()].addr().as_u64() as *mut PageTable;

            if !(*pml1)[virt.p1_index()]
                .flags()
                .contains(PageTableFlags::PRESENT)
            {
                return;
            }

            (*pml1)[virt.p1_index()].set_unused();
        }
    }
}

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        let mut mapper = Pagemap::new(None);

        mapper.map_kernel();

        debug!("mapped kernel");

        // map all memory regions in higher half
        for memory_entry in *MEMORY_MAP {
            mapper.map_range(
                PhysAddr::new(memory_entry.base),
                VirtAddr::new(memory_entry.base + *HIGHER_HALF_OFFSET),
                memory_entry.len as usize,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                false,
            )
        }

        mapper.load_pt();

        *KERNEL_PAGE_MAPPER.lock() = InitAtBoot::Initialized(mapper);
    }
}
