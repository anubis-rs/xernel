use lazy_static::lazy_static;

use limine::{LimineFramebuffer, LimineFramebufferRequest};

use crate::framebuffer::font::FONT;

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

lazy_static! {
    static ref FRAMEBUFFER: &'static LimineFramebuffer = {
        FRAMEBUFFER_REQUEST
            .get_response()
            .get()
            .expect("limine-protocol: invalid framebuffer response")
            .framebuffers
            .get()
            .expect("limine-protocol: could not get framebuffer")
            .get()
            .unwrap()
    };
}

pub unsafe fn printc(character: char) {
    let c = character as u8;

    debug_assert!(c.is_ascii());

    static mut CURSOR: u64 = 0;

    let address = FRAMEBUFFER.address.as_mut_ptr().unwrap().cast::<u8>();

    let mut index: u16 = 0;

    if character == '\n' {
        CURSOR -= CURSOR % FRAMEBUFFER.pitch;
        CURSOR += FRAMEBUFFER.pitch * 17;

        return;
    }

    if character != ' ' {
        index = (c as u16 - 32) * 16;
    }

    for i in index..index + 16 {
        let bitmap: u8 = FONT[i as usize];

        for j in 0..8 {
            if (bitmap & (1 << (7 - j))) >= 1 {
                address.add(CURSOR as usize).write_volatile(0xff);
                address.add((CURSOR + 1) as usize).write_volatile(0xff);
                address.add((CURSOR + 2) as usize).write_volatile(0xff);
            }
            CURSOR += (FRAMEBUFFER.bpp / 8) as u64;
        }
        CURSOR -= FRAMEBUFFER.bpp as u64;
        CURSOR += FRAMEBUFFER.pitch;
    }

    CURSOR += FRAMEBUFFER.bpp as u64;
    CURSOR += (FRAMEBUFFER.bpp / 8) as u64;

    CURSOR -= FRAMEBUFFER.pitch * 16;
}
