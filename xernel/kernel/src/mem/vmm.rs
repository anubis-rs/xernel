use super::HIGHER_HALF_OFFSET;
use crate::{
    mem::{pmm::FRAME_SIZE, KERNEL_OFFSET},
    print, println,
};
use x86_64::{structures::paging::FrameAllocator, registers::control::{Cr3, Cr3Flags}};
use x86_64::{
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        println!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

        let mut frame_allocator = super::pmm::FRAME_ALLOCATOR.lock();

        let lvl4_frame = frame_allocator.allocate_frame().unwrap();
        let lvl4_table = lvl4_frame.start_address().as_u64() as *mut PageTable;

        (*lvl4_table).zero();

        // the bootloader has identity mapped all memory regions
        let mut mapper = OffsetPageTable::new(&mut *lvl4_table, VirtAddr::new(0));

        let mut count = 0;

        // map the kernel
        for start_adress in (KERNEL_OFFSET..0xffff_ffff_ffff_ffff).step_by(FRAME_SIZE as usize) {
            count += 1;

            let frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(start_adress - KERNEL_OFFSET));
            let page = Page::containing_address(VirtAddr::new(start_adress));

            let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE; // TODO: remove writable

            let map_to_result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

            map_to_result.unwrap().ignore();
        }

        println!("count: {}", count);

        println!("cr3 = {:x}", lvl4_frame.start_address().as_u64());
        println!("next frame = {:x}", frame_allocator.allocate_frame().unwrap().start_address().as_u64());

        //Cr3::write(lvl4_frame, Cr3Flags::empty());
        core::arch::asm!("mov cr3, {}", in(reg) lvl4_frame.start_address().as_u64(), options(nostack, preserves_flags));

        dbg!("survived loading new page table");

        // map the kernel
        // let result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

        // println!("result: {:?}", result);

        // TODO: create new page table and map all memory maps in higher half

        // core::arch::asm!("mov rax, [0xdeadbeef]");
    }
}
