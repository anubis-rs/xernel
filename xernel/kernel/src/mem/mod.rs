pub mod frame;
pub mod heap;
pub mod mmap;
pub mod paging;
pub mod vm;

use libxernel::sync::Once;
use limine::HhdmRequest;

static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);

pub static HIGHER_HALF_OFFSET: Once<u64> = Once::new();

pub const KERNEL_OFFSET: u64 = 0xffff_ffff_8000_0000;
pub const HEAP_START_ADDR: usize = 0xffff_9000_0000_0000;
// NOTE: stack grows down
pub const KERNEL_THREAD_STACK_TOP: u64 = 0xffff_a000_0000_0000;
pub const USER_THREAD_STACK_TOP: u64 = 0x0000_ffff_ffff_f000;

pub const STACK_SIZE: u64 = 0x40000;
pub const FRAME_SIZE: u64 = 4096;

pub fn init() {
    HIGHER_HALF_OFFSET.set_once(HHDM_REQUEST.get_response().get().unwrap().offset);

    frame::init();
    info!("pm initialized");

    paging::init();
    info!("vm initialized");

    heap::init();
    info!("heap initialized");
}
