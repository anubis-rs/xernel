use x86_64::{structures::paging::{OffsetPageTable, PageTable, Mapper, PhysFrame, Size4KiB}, registers::control::Cr3, VirtAddr, PhysAddr};
use crate::{print, println};
use super::HIGHER_HALF_OFFSET;

pub fn init() {
    unsafe {
        // create new pagetable and map the kernel + all memory maps in higher half
        println!("higher half offset: {:x?}", *HIGHER_HALF_OFFSET);

        let (phys_frame, _) = Cr3::read();
        let phys = phys_frame.start_address();
        let page_table_ptr: *mut PageTable = phys.as_u64() as *mut PageTable;

        let mapper = OffsetPageTable::new(&mut *page_table_ptr, VirtAddr::new(*HIGHER_HALF_OFFSET));

        let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(0xdeadbeef));

        // map the kernel
        //mapper.map_to(page, frame, flags, frame_allocator);

        let ptr_deadbeef: *mut u64 = 0xdeadbeef as *mut u64;
        println!("deadbeef is {}", *ptr_deadbeef);
        ptr_deadbeef.write_volatile(0x1337 as u64);
        println!("deadbeef is now {}", *ptr_deadbeef);
    }
}
