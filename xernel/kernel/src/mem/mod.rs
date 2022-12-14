pub mod heap;
pub mod pmm;
pub mod vmm;

use limine::LimineHhdmRequest;

static HHDM_REQUEST: LimineHhdmRequest = LimineHhdmRequest::new(0);

lazy_static! {
    pub static ref HIGHER_HALF_OFFSET: u64 = HHDM_REQUEST.get_response().get().unwrap().offset;
}

pub const KERNEL_OFFSET: u64 = 0xffff_ffff_8000_0000;
pub const STACK_SIZE: u64 = 0x80000;
pub const FRAME_SIZE: u64 = 4096;
