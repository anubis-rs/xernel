use alloc::alloc::alloc_zeroed;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::addr_of;
use libxernel::sync::{Once, Spinlock};
use x86_64::VirtAddr;
use x86_64::instructions::segmentation::{CS, DS, ES, SS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const IST_STACK_SIZE: usize = 4096 * 5;

static mut BSP_IST_STACK: [u8; IST_STACK_SIZE] = [0; IST_STACK_SIZE];

static TSS: Once<TaskStateSegment> = Once::new();
pub static GDT_BSP: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

static GDT_AP: Spinlock<Vec<Gdt>> = Spinlock::new(Vec::new());
static TSS_MAP: Spinlock<Vec<Option<*mut TaskStateSegment>>> = Spinlock::new(Vec::new());

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
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        let stack_start = VirtAddr::from_ptr(addr_of!(BSP_IST_STACK));
        stack_start + IST_STACK_SIZE as u64
    };

    TSS.set_once(tss);

    let mut gdt = GlobalDescriptorTable::new();

    let code_selector = gdt.append(Descriptor::kernel_code_segment());
    let data_selector = gdt.append(Descriptor::kernel_data_segment());

    // let kernel_data_flags = DescriptorFlags::USER_SEGMENT | DescriptorFlags::PRESENT | DescriptorFlags::WRITABLE;
    // let data_selector = gdt.add_entry(Descriptor::UserSegment(kernel_data_flags.bits()));

    // System segment descriptors (which the TSS descriptor is) are 16-bytes and take up 2 slots in the GDT
    // This results in user code having index 5, user data index 6
    let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
    let user_data_selector = gdt.append(Descriptor::user_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());
    GDT_BSP.set_once((
        gdt,
        Selectors {
            code_selector,
            data_selector,
            tss_selector,
            user_code_selector,
            user_data_selector,
        },
    ));

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
    let code_selector = gdt.append(Descriptor::kernel_code_segment());
    let data_selector = gdt.append(Descriptor::kernel_data_segment());
    let user_data_selector = gdt.append(Descriptor::user_data_segment());
    let user_code_selector = gdt.append(Descriptor::user_code_segment());

    let mut boxed_tss = Box::new(TaskStateSegment::new());

    let ist0 = unsafe { alloc_zeroed(core::alloc::Layout::from_size_align(IST_STACK_SIZE, 4096).unwrap()) };
    boxed_tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
        unsafe { VirtAddr::from_ptr(ist0.add(IST_STACK_SIZE)) };

    let tss: &'static mut TaskStateSegment = Box::leak(boxed_tss);
    let tss_ptr = tss as *const TaskStateSegment as *mut TaskStateSegment;
    let tss_selector = gdt.append(Descriptor::tss_segment(tss));

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

    let mut tss_map = TSS_MAP.lock();
    if ap_id >= tss_map.len() {
        tss_map.resize_with(ap_id + 1, Default::default);
    }
    tss_map[ap_id] = Some(tss_ptr);

    gdt.load();
    unsafe {
        CS::set_reg(code_selector);
        SS::set_reg(data_selector);
        DS::set_reg(data_selector);
        ES::set_reg(data_selector);

        load_tss(tss_selector);
    }
}

pub fn set_tss_kernel_stack(stack_top: u64) {
    unsafe {
        let cpu_id = crate::cpu::current_cpu().cpu_id;

        // For BSP (CPU 0)
        if cpu_id == 0 {
            assert!(TSS.is_completed());
            let tss = &*TSS as *const TaskStateSegment as *mut TaskStateSegment;
            (*tss).privilege_stack_table[0] = VirtAddr::new(stack_top);
            return;
        }

        // For APs - direct lookup in TSS_MAP
        let tss_map = TSS_MAP.lock();
        if let Some(Some(tss_ptr)) = tss_map.get(cpu_id) {
            (**tss_ptr).privilege_stack_table[0] = VirtAddr::new(stack_top);
        } else {
            panic!("No TSS found for CPU {}", cpu_id);
        }
    }
}
