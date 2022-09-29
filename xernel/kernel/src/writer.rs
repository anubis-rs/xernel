use core::fmt;
use core::fmt::Write;

use libxernel::ticket::TicketMutex;

use crate::framebuffer::FRAMEBUFFER;

struct Writer;

static WRITER: TicketMutex<Writer> = TicketMutex::new(Writer);

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut fb = FRAMEBUFFER.lock();
        fb.puts(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ($crate::writer::_println(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::writer::_log_print(format_args!($($arg)*), "DEBUG", 0x03, 0xe8, 0xfc));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::writer::_log_print(format_args!($($arg)*), "INFO", 0x03, 0xfc, 0x0b));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::writer::_log_print(format_args!($($arg)*), "ERROR", 0xfc, 0x03, 0x0f));
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => ($crate::writer::_log_print(format_args!($($arg)*), "WARNING", 0xfc, 0xca, 0x03));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::writer::_print(format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    // UNWRAP: We always return `Ok(())` inside `write_str` so this is unreachable.
    writer.write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _println(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    // UNWRAP: We always return `Ok(())` inside `write_str` so this is unreachable.
    writer.write_fmt(args).unwrap();
    writer.write_char('\n').unwrap();
}

#[doc(hidden)]
pub fn _log_print(args: fmt::Arguments, level: &str, r: u8, g: u8, b: u8) {
    // UNWRAP: We always return `Ok(())` inside `write_str` so this is unreachable.
    let mut writer = WRITER.lock();

    writer.write_char('[').unwrap();

    FRAMEBUFFER.lock().set_color(r, g, b);
    writer.write_str(level).unwrap();
    FRAMEBUFFER.lock().set_color(0xff, 0xff, 0xff);
    writer.write_str("] ").unwrap();
    writer.write_fmt(args).unwrap();

    writer.write_char('\n').unwrap();
}
