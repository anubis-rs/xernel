use super::font::FONT;
use core::ptr::write_volatile;
use limine::{LimineFramebufferRequest, LimineFramebuffer};

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

static mut FRAMEBUFFER: Option<&LimineFramebuffer> = None;

pub unsafe fn init_framebuffer() {
    FRAMEBUFFER = FRAMEBUFFER_REQUEST.get_response().get().unwrap().framebuffers.get().unwrap().get();
}

pub unsafe fn printc() {
    let address = FRAMEBUFFER.unwrap().address.as_mut_ptr().unwrap();

    for i in 0..10000 {
        write_volatile(address, 0xff);
    }
    
}