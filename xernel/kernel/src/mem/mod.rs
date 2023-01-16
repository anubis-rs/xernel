pub mod heap;
pub mod pmm;
pub mod vmm;

use libxernel::sync::Once;
use limine::LimineHhdmRequest;

use crate::info;

static HHDM_REQUEST: LimineHhdmRequest = LimineHhdmRequest::new(0);

pub static HIGHER_HALF_OFFSET: Once<u64> = Once::new();

pub const KERNEL_OFFSET: u64 = 0xffff_ffff_8000_0000;
pub const STACK_SIZE: u64 = 0x80000;
pub const FRAME_SIZE: u64 = 4096;

pub fn init() {
    HIGHER_HALF_OFFSET.set_once(HHDM_REQUEST.get_response().get().unwrap().offset);

    pmm::init();
    info!("pm initialized");

    vmm::init();
    info!("vm initialized");

    heap::init();
    info!("heap initialized");
}
