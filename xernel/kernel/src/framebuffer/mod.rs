mod font;

use core::ptr::copy;

use crate::framebuffer::font::FONT;
use limine::{LimineFramebuffer, LimineFramebufferRequest};

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

lazy_static! {
    static ref FRAMEBUFFER: &'static LimineFramebuffer = {
        FRAMEBUFFER_REQUEST
            .get_response()
            .get()
            .expect("limine-protocol: invalid framebuffer response")
            .framebuffers()
            .unwrap()
            .first()
            .expect("limine-protocol: could not get first framebuffer")
    };
}

pub unsafe fn printc(character: char) {
    debug_assert!(character.is_ascii());

    let c = character as u8;

    static mut CURSOR: u64 = 0;
    static mut CHAR_LINE: u64 = 0;

    let address = FRAMEBUFFER.address.as_mut_ptr().unwrap().cast::<u8>();

    let mut index: u16 = 0;

    if CURSOR >= fb_length() - FRAMEBUFFER.pitch * 17 {
        CURSOR -= FRAMEBUFFER.pitch * 17;

        copy(address.add((FRAMEBUFFER.pitch*17) as usize), address, (fb_length() - FRAMEBUFFER.pitch * 17) as usize);

        for i in 0..FRAMEBUFFER.pitch*17 {
            address.add((CURSOR + i) as usize).write_volatile(0x00);
        }
    }

    if character == '\n' {
        CURSOR -= CURSOR % FRAMEBUFFER.pitch;
        CURSOR += FRAMEBUFFER.pitch * 17;

        CHAR_LINE = 0;

        return;
    }

    if character == '\t' {
        CURSOR += 32*4*4;

        return;
    }

    if CHAR_LINE == FRAMEBUFFER.width/9 {
        CHAR_LINE = 0;

        CURSOR -= CURSOR % FRAMEBUFFER.pitch;
        CURSOR += FRAMEBUFFER.pitch * 17;
    }

    if character != ' ' {
        index = (c as u16 - 32) * 16;
    }

    CHAR_LINE += 1;

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

fn fb_length() -> u64 {
    (FRAMEBUFFER.height * FRAMEBUFFER.pitch) as u64
}
