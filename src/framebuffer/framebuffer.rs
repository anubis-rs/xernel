use lazy_static::lazy_static;

use core::ptr::write_volatile;
use limine::{LimineFramebufferRequest, LimineFramebuffer};

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

lazy_static! {
    static ref FRAMEBUFFER: Option<&'static LimineFramebuffer> = {
        FRAMEBUFFER_REQUEST.get_response().get().unwrap().framebuffers.get().unwrap().get()
    };
}

pub fn printc(_character: char) {
    unsafe {
        let address = FRAMEBUFFER.unwrap().address.as_mut_ptr().unwrap();
        for i in 0..10000 {
            write_volatile(address, 0xff);
        }
    }
}