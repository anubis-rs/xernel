mod font;

use core::ptr::{copy, write_bytes, copy_nonoverlapping};

use crate::{framebuffer::font::FONT, limine_module::get_limine_module, mem::frame};
use libxernel::sync::{Once, Spinlock};
use limine::{LimineFile, LimineFramebuffer, LimineFramebufferRequest};
use alloc::vec::Vec;

/// A struct providing information about the framebuffer
pub struct Framebuffer {
    /// Current position (in byte) in the pixel framebuffer
    cursor: u64,
    /// How many characters were printed on the line already
    char_current_line: u8,
    /// Current selected color
    color: Color,
    /// Address where the framebuffer should print
    address: *mut u8,
    double_buffer: Vec<u8>,
}

/// Type to represent a RGB color value
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

/// [`Framebuffer`] wrapped in a [`Spinlock`] for static usage
pub static FRAMEBUFFER: Spinlock<Framebuffer> = Spinlock::new(Framebuffer {
    cursor: 0,
    char_current_line: 0,
    color: Color {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    },
    address: core::ptr::null_mut(),
    double_buffer: Vec::new(),
});

pub static FRAMEBUFFER_DATA: Once<&'static LimineFramebuffer> = Once::new();

pub fn init() {
    FRAMEBUFFER_DATA.set_once(
        FRAMEBUFFER_REQUEST
            .get_response()
            .get()
            .expect("limine-protocol: invalid framebuffer response")
            .framebuffers()
            .first()
            .expect("limine-protocol: could not get first framebuffer"),
    );

    // show start image
    let img_file = get_limine_module("logo").unwrap();

    unsafe {
        let mut framebuffer = FRAMEBUFFER.lock();

        framebuffer.address = FRAMEBUFFER_DATA
            .address
            .as_ptr()
            .expect("Could not get framebuffer address")
            .cast::<u8>();

        framebuffer.show_bitmap_image(img_file);
    }
}

pub fn late_init() {
    let mut framebuffer = FRAMEBUFFER.lock();

    framebuffer.double_buffer = Vec::with_capacity(framebuffer.length() as usize);

    for _ in 0..framebuffer.length() {
        framebuffer.double_buffer.push(0);
    }
}
impl Framebuffer {
    /// Prints a single character to the framebuffer
    ///
    /// Writes a single given character (from the included FONT) to the framebuffer
    /// Also implements the support for downscrolling the framebuffer
    unsafe fn putc(&mut self, character: char) {
        debug_assert!(character.is_ascii());

        let c = character as u8;

        let mut index: u16 = 0;

        if u64::from(self.char_current_line) == (FRAMEBUFFER_DATA.width / 9) {
            self.char_current_line = 0;

            self.cursor -= self.cursor % FRAMEBUFFER_DATA.pitch;
            self.cursor += FRAMEBUFFER_DATA.pitch * 17;
        }

        if self.cursor >= self.length() - FRAMEBUFFER_DATA.pitch * 17 {
            self.cursor -= FRAMEBUFFER_DATA.pitch * 17;

            copy(
                self.address.add((FRAMEBUFFER_DATA.pitch * 17) as usize),
                self.address,
                (self.length() - FRAMEBUFFER_DATA.pitch * 17) as usize,
            );

            for i in 0..FRAMEBUFFER_DATA.pitch * 17 {
                self.address
                    .add((self.cursor + i) as usize)
                    .write_volatile(0x00);
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
                    self.address
                        .add(self.cursor as usize)
                        .write_volatile(self.color.b);
                    self.address
                        .add((self.cursor + 1) as usize)
                        .write_volatile(self.color.g);
                    self.address
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
    /// Iterates over a string and calls [`Framebuffer::putc`] for every character.
    pub fn puts(&mut self, string: &str) {
        unsafe {
            for c in string.chars() {
                self.putc(c);
            }
        }
    }

    /// Returns the framebuffer size in bytes
    pub fn length(&self) -> u64 {
        FRAMEBUFFER_DATA.height * FRAMEBUFFER_DATA.pitch
    }

    pub fn clear_screen(&mut self) {
        unsafe {
            copy_nonoverlapping(self.double_buffer.as_ptr(), self.address, self.length() as usize);
        }

        self.double_buffer.fill(0);
    }

    pub fn dimensions(&self) -> (u16, u16) {
        (FRAMEBUFFER_DATA.width as u16, FRAMEBUFFER_DATA.height as u16)
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, r: i32, g: i32, b: i32) {
        assert!(y1 == y2);

        if (y1 < 0) || (y1 >= FRAMEBUFFER_DATA.height as i32) {
            return;
        }

        if (x1 < 0 || x1 >= FRAMEBUFFER_DATA.width as i32)
            || (x2 < 0 || x2 >= FRAMEBUFFER_DATA.width as i32)
        {
            return;
        }

        dbg!("({},{}) -> ({},{})", x1, y1, x2, y2);

        for x in x1..(x2 + 1) {
            // draw pixel
            let pixel_count = (y1 as u64 * FRAMEBUFFER_DATA.width) + x as u64;
            let pixel_count = pixel_count * (FRAMEBUFFER_DATA.bpp / 8) as u64;

            self.double_buffer[pixel_count as usize] = b as u8;
            self.double_buffer[(pixel_count + 1) as usize] = g as u8;
            self.double_buffer[(pixel_count + 2) as usize] = r as u8;
        }
    }

    /// Sets the color which the framebuffer uses for writing
    ///
    /// Accepts three [`u8`] arguments which represent the values of the rgb color model
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.color.r = r;
        self.color.g = g;
        self.color.b = b;
    }

    /// Sets the cursor to the start of the next pixel line
    pub fn new_line(&mut self) {
        self.cursor -= self.cursor % FRAMEBUFFER_DATA.pitch;
        self.cursor += FRAMEBUFFER_DATA.pitch;
    }

    /// Displays a given bitmap image on the framebuffer
    pub unsafe fn show_bitmap_image(&mut self, image_data: &LimineFile) {
        let address = FRAMEBUFFER_DATA.address.as_ptr().unwrap().cast::<u8>();

        let file_base = image_data.base.as_ptr().unwrap();

        let bpp = file_base.offset(0x1c).read();

        let img_data_offset = file_base.offset(0xa).read() as u32;

        let img_base = file_base.add(img_data_offset as usize);

        let mut image_addr = img_base;

        let width = file_base.offset(0x12).read() as u16;
        let height = file_base.offset(0x16).read() as u16;

        self.new_line();

        for i in 0..(width * height) {
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
            self.cursor += FRAMEBUFFER_DATA.bpp as u64 / 8;

            if i % width == 0 && i != 0 {
                self.new_line();
            }
        }

        self.new_line();
        self.new_line();
    }
}
