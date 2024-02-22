use super::{frame::FRAME_ALLOCATOR, HIGHER_HALF_OFFSET};
use crate::{
    allocator::align_up,
    mem::{frame::MEMORY_MAP, KERNEL_OFFSET},
};
use libxernel::boot::InitAtBoot;
use libxernel::sync::Spinlock;
use limine::KernelAddressRequest;
use x86_64::{
    align_down,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{Page, PageSize, PageTableIndex, Size1GiB, Size2MiB, Size4KiB},
};
use x86_64::{
    structures::paging::{PageTable, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

static KERNEL_ADDRESS_REQUEST: KernelAddressRequest = KernelAddressRequest::new(0);

pub static KERNEL_PAGE_MAPPER: Spinlock<InitAtBoot<Pagemap>> = Spinlock::new(InitAtBoot::Uninitialized);

#[derive(Debug, Clone)]
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
        let frame = pt_frame.unwrap_or_else(|| {
            let mut frame_allocator = FRAME_ALLOCATOR.lock();

            frame_allocator.allocate_frame().unwrap()
        });

        let pt_address = unsafe {
            let ptr = (frame.start_address().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;
            *ptr = PageTable::new();

            // only zero if the page table was not provided
            if pt_frame.is_none() {
                (*ptr).zero();
            }

            ptr
        };

        Self { page_table: pt_address }
    }

    pub fn pml4(&self) -> PhysAddr {
        PhysAddr::new(self.page_table as u64 - *HIGHER_HALF_OFFSET)
    }

    pub fn fill_with_kernel_entries(&mut self) {
        let kernel_mapper = KERNEL_PAGE_MAPPER.lock();
        let pt = unsafe { &*kernel_mapper.page_table };

        for i in 256..512 {
            unsafe {
                (*self.page_table)[i] = pt[i].clone();
            }
        }
    }

    pub fn map<P: PageSize>(&mut self, phys: PhysFrame<P>, virt: Page<P>, flags: PageTableFlags, flush_tlb: bool) {
        let pml4 = self.page_table;

        let mut frame_allocator = FRAME_ALLOCATOR.lock();

        unsafe {
            let pml4_entry = &mut (*pml4)[virt.start_address().p4_index()];

            if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml4_entry.set_addr(PhysAddr::new(address), flags);
            }

            pml4_entry.set_flags(pml4_entry.flags() | flags);

            let pml3 = (pml4_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml3_entry = &mut (*pml3)[virt.start_address().p3_index()];

            if P::SIZE == Size1GiB::SIZE {
                assert!(
                    u16::from(virt.start_address().p2_index()) == 0
                        && u16::from(virt.start_address().p1_index()) == 0
                        && u16::from(virt.start_address().page_offset()) == 0
                );

                pml3_entry.set_addr(phys.start_address(), flags | PageTableFlags::HUGE_PAGE);

                if flush_tlb {
                    self.flush(virt.start_address());
                }

                return;
            }

            if !pml3_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml3_entry.set_addr(PhysAddr::new(address), flags);
            }

            pml3_entry.set_flags(pml4_entry.flags() | flags);

            let pml2 = (pml3_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml2_entry = &mut (*pml2)[virt.start_address().p2_index()];

            if P::SIZE == Size2MiB::SIZE {
                assert!(
                    u16::from(virt.start_address().p1_index()) == 0
                        && u16::from(virt.start_address().page_offset()) == 0
                );

                pml2_entry.set_addr(phys.start_address(), flags | PageTableFlags::HUGE_PAGE);

                if flush_tlb {
                    self.flush(virt.start_address());
                }

                return;
            }

            if !pml2_entry.flags().contains(PageTableFlags::PRESENT) {
                let frame = frame_allocator.allocate_frame::<Size4KiB>().unwrap();

                let address = frame.start_address().as_u64();

                let p_table: *mut PageTable = (address + *HIGHER_HALF_OFFSET) as *mut PageTable;
                *p_table = PageTable::new();

                pml2_entry.set_addr(PhysAddr::new(address), flags);
            }

            pml2_entry.set_flags(pml4_entry.flags() | flags);

            let pml1 = (pml2_entry.addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

            let pml1_entry = &mut (*pml1)[virt.start_address().p1_index()];

            pml1_entry.set_addr(phys.start_address(), flags);

            if flush_tlb {
                self.flush(virt.start_address());
            }
        }
    }

    pub fn flush(&self, addr: VirtAddr) {
        unsafe {
            core::arch::asm!("invlpg [{}]", in(reg) addr.as_u64(), options(nostack, preserves_flags));
        }
    }

    pub fn map_range(&mut self, phys: PhysAddr, virt: VirtAddr, amount: usize, flags: PageTableFlags, flush_tlb: bool) {
        assert!(u16::from(virt.page_offset()) == 0);
        assert!(phys.is_aligned(Size4KiB::SIZE));

        let aligned_amount = align_up(amount, Size4KiB::SIZE as usize);

        let mut offset: u64 = 0;

        // map all 4kib pages till 2mib aligned
        let pages_4kb = (virt.align_up(Size2MiB::SIZE).as_u64() - virt.as_u64()) / Size4KiB::SIZE;

        for _ in 0..pages_4kb {
            if offset >= aligned_amount as u64 {
                break;
            }

            self.map::<Size4KiB>(
                PhysFrame::from_start_address(phys + offset).unwrap(),
                Page::from_start_address(virt + offset).unwrap(),
                flags,
                flush_tlb,
            );

            offset += Size4KiB::SIZE;
        }

        // map all 2mib pages
        let pages_2mb = align_down(aligned_amount as u64 - offset, Size2MiB::SIZE) / Size2MiB::SIZE;

        for _ in 0..pages_2mb {
            self.map::<Size2MiB>(
                PhysFrame::from_start_address(phys + offset).unwrap(),
                Page::from_start_address(virt + offset).unwrap(),
                flags,
                flush_tlb,
            );

            offset += Size2MiB::SIZE;
        }

        // map 4kib pages till the end
        let pages_4kb = align_up(aligned_amount - offset as usize, Size4KiB::SIZE as usize) / Size4KiB::SIZE as usize;

        for _ in 0..pages_4kb {
            self.map::<Size4KiB>(
                PhysFrame::from_start_address(phys + offset).unwrap(),
                Page::from_start_address(virt + offset).unwrap(),
                flags,
                flush_tlb,
            );

            offset += Size4KiB::SIZE;
        }
    }

    pub unsafe fn load_pt(&self) {
        let pt = self.page_table;
        let phys = pt as *const _ as u64 - *HIGHER_HALF_OFFSET;

        Cr3::write(
            PhysFrame::from_start_address(PhysAddr::new(phys)).unwrap(),
            Cr3Flags::empty(),
        );
    }

    pub fn map_kernel(&mut self) {
        debug!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

        let kernel_base_address = KERNEL_ADDRESS_REQUEST.get_response().get().unwrap().physical_base;
        let kernel_virt_address = KERNEL_ADDRESS_REQUEST.get_response().get().unwrap().virtual_base;

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
            if !(*pml4)[virt.p4_index()].flags().contains(PageTableFlags::PRESENT) {
                return;
            }

            let pml3 = (*pml4)[virt.p4_index()].addr().as_u64() as *mut PageTable;

            if !(*pml3)[virt.p3_index()].flags().contains(PageTableFlags::PRESENT) {
                return;
            }

            let pml2 = (*pml3)[virt.p3_index()].addr().as_u64() as *mut PageTable;

            if !(*pml2)[virt.p2_index()].flags().contains(PageTableFlags::PRESENT) {
                return;
            }

            let pml1 = (*pml2)[virt.p2_index()].addr().as_u64() as *mut PageTable;

            if !(*pml1)[virt.p1_index()].flags().contains(PageTableFlags::PRESENT) {
                return;
            }

            (*pml1)[virt.p1_index()].set_unused();
        }
    }

    unsafe fn get_pt(pt: *mut PageTable, pt_index: PageTableIndex) -> *mut PageTable {
        ((*pt)[pt_index].addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable
    }

    /// Only works with 4KiB pages
    pub fn translate(&self, virt: VirtAddr) -> Option<PhysAddr> {
        let pml4 = self.page_table;

        unsafe {
            if !(*pml4)[virt.p4_index()].flags().contains(PageTableFlags::PRESENT) {
                return None;
            }

            let pml3 = Self::get_pt(pml4, virt.p4_index());

            if !(*pml3)[virt.p3_index()].flags().contains(PageTableFlags::PRESENT) {
                return None;
            }

            let pml2 = Self::get_pt(pml3, virt.p3_index());

            if !(*pml2)[virt.p2_index()].flags().contains(PageTableFlags::PRESENT) {
                return None;
            }

            let pml1 = Self::get_pt(pml2, virt.p2_index());

            if !(*pml1)[virt.p1_index()].flags().contains(PageTableFlags::PRESENT) {
                return None;
            }

            Some((*pml1)[virt.p1_index()].addr() + u64::from(virt.page_offset()))
        }
    }

    pub fn deallocate_pt(pt: *mut PageTable, level: u8) {
        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        if level == 4 {
            for i in 0..256 {
                unsafe {
                    if (*pt)[i].flags().contains(PageTableFlags::PRESENT) {
                        let pt = ((*pt)[i].addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

                        Self::deallocate_pt(pt, level - 1);
                    }
                }
            }

            unsafe {
                frame_allocator.deallocate_frame(
                    PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(pt as u64 - *HIGHER_HALF_OFFSET)).unwrap(),
                );
            }
        } else if level > 1 {
            for i in 0..512 {
                unsafe {
                    if (*pt)[i].flags().contains(PageTableFlags::PRESENT) {
                        let pt = ((*pt)[i].addr().as_u64() + *HIGHER_HALF_OFFSET) as *mut PageTable;

                        Self::deallocate_pt(pt, level - 1);
                    }
                }
            }

            unsafe {
                frame_allocator.deallocate_frame(
                    PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(pt as u64 - *HIGHER_HALF_OFFSET)).unwrap(),
                );
            }
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
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
                false,
            )
        }

        mapper.load_pt();

        *KERNEL_PAGE_MAPPER.lock() = InitAtBoot::Initialized(mapper);
    }
}
