use super::HIGHER_HALF_OFFSET;
use crate::{
    mem::{pmm::FRAME_SIZE, KERNEL_OFFSET},
    print, println,
};
use limine::LimineKernelAddressRequest;
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::FrameAllocator,
};
use x86_64::{
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

static KERNEL_ADDRESS_REQUEST: LimineKernelAddressRequest = LimineKernelAddressRequest::new(0);

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        println!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

        let mut frame_allocator = super::pmm::FRAME_ALLOCATOR.lock();

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

        let lvl4_frame = frame_allocator.allocate_frame().unwrap();
        let lvl4_table = lvl4_frame.start_address().as_u64() as *mut PageTable;

        (*lvl4_table).zero();

        // the bootloader has identity mapped all memory regions
        let mut mapper = OffsetPageTable::new(&mut *lvl4_table, VirtAddr::new(0));

        let mut count = 0;

        for address in (0..0x100000000).step_by(FRAME_SIZE as usize) {
            count += 1;

            let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(address));
            let page = Page::containing_address(VirtAddr::new(address + *HIGHER_HALF_OFFSET));

            let flags = PageTableFlags::PRESENT
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::WRITABLE;

            let map_to_result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

            map_to_result.unwrap().ignore();
        }

        for address in (0..0x80000000).step_by(FRAME_SIZE as usize) {
            count += 1;

            let frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(address + kernel_base_address));
            let page = Page::containing_address(VirtAddr::new(address + KERNEL_OFFSET));

            let flags = PageTableFlags::PRESENT
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::WRITABLE;

            let map_to_result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

            map_to_result.unwrap().ignore();
        }

        // map all memory regions in higher half
        for memory_entry in frame_allocator.mmap {
            for start_adress in (memory_entry.base..memory_entry.base + memory_entry.len)
                .step_by(FRAME_SIZE as usize)
            {
                let frame: PhysFrame<Size4KiB> =
                    PhysFrame::containing_address(PhysAddr::new(start_adress));
                let page =
                    Page::containing_address(VirtAddr::new(start_adress + *HIGHER_HALF_OFFSET));

                let flags = PageTableFlags::PRESENT
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::WRITABLE;

                let map_to_result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

                if let Ok(map_to) = map_to_result {
                    map_to.ignore();
                }
            }
        }

        println!("count: {}", count);

        println!("cr3 = {:x}", lvl4_frame.start_address().as_u64());
        println!(
            "next frame = {:x}",
            frame_allocator
                .allocate_frame()
                .unwrap()
                .start_address()
                .as_u64()
        );

        Cr3::write(lvl4_frame, Cr3Flags::empty());

        dbg!("new page table loaded");
    }
}
