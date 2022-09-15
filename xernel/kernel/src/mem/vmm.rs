use super::{pmm::FRAME_ALLOCATOR, HIGHER_HALF_OFFSET};
use crate::{
    mem::{
        pmm::{FRAME_SIZE, MEMORY_MAP},
        KERNEL_OFFSET,
    },
    print, println,
};
use libxernel::boot::InitAtBoot;
use libxernel::spin::Spinlock;
use limine::LimineKernelAddressRequest;
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{mapper::MapToError, FrameAllocator},
};
use x86_64::{
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

static KERNEL_ADDRESS_REQUEST: LimineKernelAddressRequest = LimineKernelAddressRequest::new(0);

pub static KERNEL_PAGE_MAPPER: Spinlock<InitAtBoot<PageMapper>> = Spinlock::new(InitAtBoot::Uninitialized);

pub struct PageMapper<'a> {
    offset_pt: OffsetPageTable<'a>,
}

impl PageMapper<'_> {
    pub fn new(lvl4_table: PhysFrame, zero_out_frame: bool) -> Self {
        let page_table = unsafe { &mut *(lvl4_table.start_address().as_u64() as *mut PageTable) };

        if zero_out_frame {
            page_table.zero();
        }

        Self {
            offset_pt: unsafe {
                OffsetPageTable::new(page_table, VirtAddr::new(*HIGHER_HALF_OFFSET))
            },
        }
    }

    pub unsafe fn map(
        &mut self,
        phys: PhysAddr,
        virt: VirtAddr,
        flags: PageTableFlags,
        flush_tlb: bool,
    ) -> Result<(), MapToError<Size4KiB>> {
        let frame = PhysFrame::containing_address(phys);
        let page = Page::containing_address(virt);

        let result = self
            .offset_pt
            .map_to(page, frame, flags, &mut *FRAME_ALLOCATOR.lock())?;

        if flush_tlb {
            result.flush();
        } else {
            result.ignore();
        }

        Ok(())
    }

    pub unsafe fn map_range(
        &mut self,
        phys: PhysAddr,
        virt: VirtAddr,
        amount: usize,
        flags: PageTableFlags,
        flush_tlb: bool,
    ) -> Result<(), MapToError<Size4KiB>> {
        let mut pages_to_map = amount as u64 / FRAME_SIZE;

        if amount as u64 % FRAME_SIZE != 0 {
            pages_to_map += 1;
        }

        let mut frame_allocator = FRAME_ALLOCATOR.lock();

        for i in 0..pages_to_map {
            let frame = PhysFrame::containing_address(phys + i as u64 * FRAME_SIZE);
            let page = Page::containing_address(virt + i as u64 * FRAME_SIZE);

            let result = self
                .offset_pt
                .map_to(page, frame, flags, &mut *frame_allocator)?;

            if flush_tlb {
                result.flush();
            } else {
                result.ignore();
            }
        }

        Ok(())
    }

    pub unsafe fn load_pt(&mut self) {
        let pt = self.offset_pt.level_4_table();
        let phys = pt as *const _ as u64;

        Cr3::write(
            PhysFrame::from_start_address(PhysAddr::new(phys)).unwrap(),
            Cr3Flags::empty(),
        );
    }
}

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        println!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

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

        println!("{:x}", kernel_base_address);
        println!("{:x}", kernel_virt_address);

        let mut frame_allocator = super::pmm::FRAME_ALLOCATOR.lock();
        let lvl4_table = frame_allocator.allocate_frame().unwrap();
        drop(frame_allocator);

        let mut mapper = PageMapper::new(lvl4_table, true);

        // TODO: calculate amount and not hardcode
        mapper
            .map_range(
                PhysAddr::new(kernel_base_address),
                VirtAddr::new(KERNEL_OFFSET),
                0x800000,
                PageTableFlags::PRESENT
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::WRITABLE,
                false,
            )
            .unwrap();

        // map all memory regions in higher half
        for memory_entry in *MEMORY_MAP {
            mapper
                .map_range(
                    PhysAddr::new(memory_entry.base),
                    VirtAddr::new(memory_entry.base + *HIGHER_HALF_OFFSET),
                    memory_entry.len as usize,
                    PageTableFlags::PRESENT
                        | PageTableFlags::USER_ACCESSIBLE
                        | PageTableFlags::WRITABLE,
                    false,
                )
                .unwrap();
        }

        mapper.load_pt();

        *KERNEL_PAGE_MAPPER.lock() = InitAtBoot::Initialized(mapper);

        dbg!("new page table loaded");
    }
}
