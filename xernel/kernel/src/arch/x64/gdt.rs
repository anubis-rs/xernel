use alloc::boxed::Box;
use alloc::vec::Vec;
use libxernel::ticket::TicketMutex;
use x86_64::instructions::segmentation::{Segment, CS, DS, ES, FS, GS, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss
    };
}

lazy_static! {
    static ref GDT_BSP: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());

        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
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
struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    GDT_BSP.0.load();
    unsafe {
        CS::set_reg(GDT_BSP.1.code_selector);
        SS::set_reg(GDT_BSP.1.data_selector);
        DS::set_reg(GDT_BSP.1.data_selector);
        ES::set_reg(GDT_BSP.1.data_selector);
        FS::set_reg(GDT_BSP.1.data_selector);
        GS::set_reg(GDT_BSP.1.data_selector);

        load_tss(GDT_BSP.1.tss_selector);
    }
}

pub fn init_ap(ap_id: usize) {
    let mut gdt_ap = GDT_AP.lock();

    let gdt: &'static mut GlobalDescriptorTable = Box::leak(Box::new(GlobalDescriptorTable::new()));
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
    gdt.add_entry(Descriptor::user_code_segment());
    gdt.add_entry(Descriptor::user_data_segment());

    let tss: &'static mut TaskStateSegment = Box::leak(Box::new(TaskStateSegment::new()));
    // TODO: set the interrupt stack to be able to handle double faults
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));

    gdt_ap.push(Gdt {
        gdt,
        selectors: Selectors {
            code_selector,
            data_selector,
            tss_selector,
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
        FS::set_reg(data_selector);
        GS::set_reg(data_selector);

        load_tss(tss_selector);
    }
}
