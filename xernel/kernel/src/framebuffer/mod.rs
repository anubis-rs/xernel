mod font;

use core::ptr::copy;

use crate::framebuffer::font::FONT;
use libxernel::ticket::TicketMutex;
use limine::{LimineFramebuffer, LimineFramebufferRequest};

pub struct Framebuffer {
    cursor: u64,
    char_current_line: u8,
    color: Color,
}

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

pub static FRAMEBUFFER: TicketMutex<Framebuffer> = TicketMutex::new(Framebuffer {
    cursor: 0,
    char_current_line: 0,
    color: Color {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    },
});

lazy_static! {
    static ref FRAMEBUFFER_DATA: &'static LimineFramebuffer = {
        FRAMEBUFFER_REQUEST
                .get_response()
                .get()
                .expect("limine-protocol: invalid framebuffer response")
                .framebuffers()
                .first()
                .expect("limine-protocol: could not get first framebuffer")
    };


}

impl Framebuffer {
    unsafe fn putc(&mut self, character: char) {
        debug_assert!(character.is_ascii());

        let c = character as u8;

        let address = FRAMEBUFFER_DATA.address.as_ptr().unwrap().cast::<u8>();

        let mut index: u16 = 0;

        if u64::from(self.char_current_line) == (FRAMEBUFFER_DATA.width / 9) {
            self.char_current_line = 0;

            self.cursor -= self.cursor % FRAMEBUFFER_DATA.pitch;
            self.cursor += FRAMEBUFFER_DATA.pitch * 17;
        }

        if self.cursor >= self.length() - FRAMEBUFFER_DATA.pitch * 17 {
            self.cursor -= FRAMEBUFFER_DATA.pitch * 17;

            copy(
                address.add((FRAMEBUFFER_DATA.pitch * 17) as usize),
                address,
                (self.length() - FRAMEBUFFER_DATA.pitch * 17) as usize,
            );

            for i in 0..FRAMEBUFFER_DATA.pitch * 17 {
                address.add((self.cursor + i) as usize).write_volatile(0x00);
            }
        }

        if character == '\n' {
            self.cursor -= self.cursor % FRAMEBUFFER_DATA.pitch;
            self.cursor += FRAMEBUFFER_DATA.pitch * 17;

            self.char_current_line = 0;

            return;
        }

        if character == '\t' {
            self.cursor += 32 * 4 * 4;

            return;
        }

        if character != ' ' {
            index = (c as u16 - 32) * 16;
        }

        self.char_current_line += 1;

        for i in index..index + 16 {
            let bitmap: u8 = FONT[i as usize];

            for j in 0..8 {
                if (bitmap & (1 << (7 - j))) >= 1 {
                    address
                        .add(self.cursor as usize)
                        .write_volatile(self.color.b);
                    address
                        .add((self.cursor + 1) as usize)
                        .write_volatile(self.color.g);
                    address
                        .add((self.cursor + 2) as usize)
                        .write_volatile(self.color.r);
                }
                self.cursor += (FRAMEBUFFER_DATA.bpp / 8) as u64;
            }
            self.cursor -= FRAMEBUFFER_DATA.bpp as u64;
            self.cursor += FRAMEBUFFER_DATA.pitch;
        }

        self.cursor += FRAMEBUFFER_DATA.bpp as u64;
        self.cursor += (FRAMEBUFFER_DATA.bpp / 8) as u64;

        self.cursor -= FRAMEBUFFER_DATA.pitch * 16;
    }

    pub fn puts(&mut self, string: &str) {
        unsafe {
            for c in string.chars() {
                self.putc(c);
            }
        }
    }

    pub fn length(&self) -> u64 {
        (FRAMEBUFFER_DATA.height * FRAMEBUFFER_DATA.pitch) as u64
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color.r = r;
        self.color.g = g;
        self.color.b = b;
    }
}
