mod font;

use core::ptr::copy;

use crate::framebuffer::font::FONT;
use libxernel::spin::Spinlock;
use limine::{LimineFramebuffer, LimineFramebufferRequest};

struct Framebuffer {
    cursor: u64,
    char_current_line: u8,
}

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

static FRAMEBUFFER: Spinlock<Framebuffer> = Spinlock::new(Framebuffer {
    cursor: 0,
    char_current_line: 0,
});

lazy_static! {
    static ref FRAMEBUFFER_DATA: &'static LimineFramebuffer = {
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

    let mut fb = FRAMEBUFFER.lock();

    let c = character as u8;

    let address = FRAMEBUFFER_DATA.address.as_mut_ptr().unwrap().cast::<u8>();

    let mut index: u16 = 0;

    if u64::from(fb.char_current_line) == (FRAMEBUFFER_DATA.width / 9) {
        fb.char_current_line = 0;

        fb.cursor -= fb.cursor % FRAMEBUFFER_DATA.pitch;
        fb.cursor += FRAMEBUFFER_DATA.pitch * 17;
    }

    if fb.cursor >= fb_length() - FRAMEBUFFER_DATA.pitch * 17 {
        fb.cursor -= FRAMEBUFFER_DATA.pitch * 17;

        copy(
            address.add((FRAMEBUFFER_DATA.pitch * 17) as usize),
            address,
            (fb_length() - FRAMEBUFFER_DATA.pitch * 17) as usize,
        );

        for i in 0..FRAMEBUFFER_DATA.pitch * 17 {
            address.add((fb.cursor + i) as usize).write_volatile(0x00);
        }
    }

    if character == '\n' {
        fb.cursor -= fb.cursor % FRAMEBUFFER_DATA.pitch;
        fb.cursor += FRAMEBUFFER_DATA.pitch * 17;

        fb.char_current_line = 0;

        return;
    }

    if character == '\t' {
        fb.cursor += 32 * 4 * 4;

        return;
    }

    if character != ' ' {
        index = (c as u16 - 32) * 16;
    }

    fb.char_current_line += 1;

    for i in index..index + 16 {
        let bitmap: u8 = FONT[i as usize];

        for j in 0..8 {
            if (bitmap & (1 << (7 - j))) >= 1 {
                address.add(fb.cursor as usize).write_volatile(0xff);
                address.add((fb.cursor + 1) as usize).write_volatile(0xff);
                address.add((fb.cursor + 2) as usize).write_volatile(0xff);
            }
            fb.cursor += (FRAMEBUFFER_DATA.bpp / 8) as u64;
        }
        fb.cursor -= FRAMEBUFFER_DATA.bpp as u64;
        fb.cursor += FRAMEBUFFER_DATA.pitch;
    }

    fb.cursor += FRAMEBUFFER_DATA.bpp as u64;
    fb.cursor += (FRAMEBUFFER_DATA.bpp / 8) as u64;

    fb.cursor -= FRAMEBUFFER_DATA.pitch * 16;
}

fn fb_length() -> u64 {
    (FRAMEBUFFER_DATA.height * FRAMEBUFFER_DATA.pitch) as u64
}
