use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::vec::Vec;
use libxernel::sync::TicketMutex;
use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const IST_STACK_SIZE: usize = 4096 * 5;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = IST_STACK_SIZE;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss
    };
}

lazy_static! {
    pub static ref GDT_BSP: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());

        // let kernel_data_flags = DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT | DescriptorFlags::WRITABLE;
        // let data_selector = gdt.add_entry(Descriptor::UserSegment(kernel_data_flags.bits()));

        // System segment descriptors (which the TSS descriptor is) are 16-bytes and take up 2 slots in the GDT
        // This results in user code having index 5, user data index 6
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
                user_code_selector,
                user_data_selector,
            },
        )
    };
}

static GDT_AP: TicketMutex<Vec<Gdt>> = TicketMutex::new(Vec::new());

#[derive(Debug)]
struct Gdt {
    gdt: &'static GlobalDescriptorTable,
    selectors: Selectors,
    tss: &'static TaskStateSegment,
    ap_id: usize,
}

#[derive(Debug)]
pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
}

pub fn init() {
    GDT_BSP.0.load();
    unsafe {
        CS::set_reg(GDT_BSP.1.code_selector);
        SS::set_reg(GDT_BSP.1.data_selector);
        DS::set_reg(GDT_BSP.1.data_selector);
        ES::set_reg(GDT_BSP.1.data_selector);

        load_tss(GDT_BSP.1.tss_selector);
    }
}

pub fn init_ap(ap_id: usize) {
    let mut gdt_ap = GDT_AP.lock();

    let gdt: &'static mut GlobalDescriptorTable = Box::leak(Box::new(GlobalDescriptorTable::new()));
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
    let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
    let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());

    let mut boxed_tss = Box::new(TaskStateSegment::new());

    let ist0 = unsafe {
        alloc_zeroed(core::alloc::Layout::from_size_align(IST_STACK_SIZE, 4096).unwrap())
    };
    boxed_tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
        unsafe { VirtAddr::from_ptr(ist0.add(IST_STACK_SIZE)) };

    let tss: &'static mut TaskStateSegment = Box::leak(boxed_tss);
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));

    gdt_ap.push(Gdt {
        gdt,
        selectors: Selectors {
            code_selector,
            data_selector,
            tss_selector,
            user_code_selector,
            user_data_selector,
        },
        tss,
        ap_id,
    });

    gdt.load();
    unsafe {
        CS::set_reg(code_selector);
        SS::set_reg(data_selector);
        DS::set_reg(data_selector);
        ES::set_reg(data_selector);

        load_tss(tss_selector);
    }
}
