use super::HIGHER_HALF_OFFSET;
use crate::{print, println};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        println!("higher half offset: {:x}", *HIGHER_HALF_OFFSET);

        let (phys_frame, _) = Cr3::read();
        let phys = phys_frame.start_address();
        let page_table_ptr: *mut PageTable = phys.as_u64() as *mut PageTable;

        let mut mapper =
            OffsetPageTable::new(&mut *page_table_ptr, VirtAddr::new(*HIGHER_HALF_OFFSET));

        let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(0x1000));
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::WRITABLE;
        let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(0xdeadbeef));

        let mut frame_allocator = super::pmm::FRAME_ALLOCATOR.lock();

        // map the kernel
        let result = mapper.map_to(page, frame, flags, &mut *frame_allocator);

        println!("result: {:?}", result);

        // TODO: create new page table and map all memory maps in higher half

        // core::arch::asm!("mov rax, [0xdeadbeef]");
    }
}
