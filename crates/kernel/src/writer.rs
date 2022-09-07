use core::fmt;
use core::fmt::Write;

use crate::framebuffer::printc;
struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            unsafe {
                printc(c);
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::writer::_print(format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = Writer;
    // UNWRAP: We always return `Ok(())` inside `write_str` so this is unreachable.
    writer.write_fmt(args).unwrap();
}
