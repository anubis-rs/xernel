mod font;

use core::ptr::copy;

use crate::{dbg, framebuffer::font::FONT};
use libxernel::ticket::TicketMutex;
use limine::{
    LimineFile, LimineFramebuffer, LimineFramebufferRequest, LimineModuleRequest, NonNullPtr,
};

/// A struct providing information about the framebuffer
pub struct Framebuffer {
    cursor: u64,
    char_current_line: u8,
    color: Color,
}

/// Type to represent a RGB color value
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);
static MODULE_REQUEST: LimineModuleRequest = LimineModuleRequest::new(0);

/// [`Framebuffer`] wrapped in a [`TicketMutex`] for static usage
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
    /// Prints a single character to the framebuffer
    ///
    /// Writes a single given character (from the included FONT) to the framebuffer
    /// Also implements the support for downscrolling the framebuffer
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

    /// Prints a string to the framebuffer
    ///
    /// Iterates over a string and calls [`putc`] for every character.
    pub fn puts(&mut self, string: &str) {
        unsafe {
            for c in string.chars() {
                self.putc(c);
            }
        }
    }

    /// Returns the framebuffer size in bytes
    pub fn length(&self) -> u64 {
        (FRAMEBUFFER_DATA.height * FRAMEBUFFER_DATA.pitch) as u64
    }

    /// Sets the color which the framebuffer uses for writing
    ///
    /// Accepts three [`u8`] arguments which represent the values of the rgb color model
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color.r = r;
        self.color.g = g;
        self.color.b = b;
    }

    // FIXME: Image weird rotated, emtpy space after image
    /// Displays a given bitmap image on the framebuffer
    pub unsafe fn show_bitmap_image(&mut self, image_data: &NonNullPtr<LimineFile>) {
        let address = FRAMEBUFFER_DATA.address.as_ptr().unwrap().cast::<u8>();

        let file_base = image_data.base.as_ptr().unwrap();

        let bpp = file_base.offset(0x1c).read() as u8;

        let img_data_offset = file_base.offset(0xA).read() as usize;

        let img_base = file_base.add(img_data_offset);

        let mut image_addr = img_base;

        let width = file_base.offset(0x12).read() as u64;
        let height = file_base.offset(0x16).read() as u64;

        dbg!("width: {}", width);
        dbg!("height: {}", height);
        dbg!("bpp: {}", bpp);

        for i in 0..(width * height * (bpp as u64 / 8)) {
            address
                .add(self.cursor as usize)
                .write_volatile(image_addr.offset(0).read());
            address
                .add((self.cursor + 1) as usize)
                .write_volatile(image_addr.offset(1).read());
            address
                .add((self.cursor + 2) as usize)
                .write_volatile(image_addr.offset(2).read());

            image_addr = image_addr.add((bpp / 8).into());
            self.cursor += bpp as u64 / 8;

            if i % width == 0 && i != 0 {
                self.cursor += FRAMEBUFFER_DATA.pitch;
                self.cursor -= width * (bpp as u64 / 8) as u64;
            }
        }

        self.cursor -= width * (bpp as u64 / 8) as u64;
        self.cursor += FRAMEBUFFER_DATA.pitch;
    }
}
/// Shows a given image included in the source code at the top of the framebuffer at the beginning
///
/// Intended as a nice gimmick so we show our logo when starting the kernel
pub fn show_start_image() {
    let module = MODULE_REQUEST
        .get_response()
        .get()
        .unwrap()
        .modules()
        .get(0)
        .unwrap();
    dbg!("module base: {:p}", module.base.as_ptr().unwrap());

    unsafe {
        let mut framebuffer = FRAMEBUFFER.lock();

        framebuffer.show_bitmap_image(module);
    }
}
