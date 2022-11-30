// Credits to Stupremee (https://github.com/Stupremee)
// https://github.com/Stupremee/novos/blob/main/crates/kernel/src/unit.rs

//! Utilities for working with raw byte units.
use core::fmt;

/// `1 KiB`
pub const KIB: usize = 1 << 10;
/// `1 MiB`
pub const MIB: usize = 1 << 20;
/// `1 GiB`
pub const GIB: usize = 1 << 30;
/// `1 TiB`
pub const TIB: usize = 1 << 40;

/// Return a formattable type that will pretty-print the given amount of bytes.
pub fn bytes<I: Into<usize> + Copy>(x: I) -> impl fmt::Display {
    ByteUnit(x)
}

/// Wrapper around raw byte that pretty-prints
/// them using the [`Display`](core::fmt::Display)
/// implementation.
#[derive(Debug, Clone, Copy)]
pub struct ByteUnit<I>(I);

impl<I> fmt::Display for ByteUnit<I>
where
    I: Into<usize> + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let this = Into::<usize>::into(self.0);
        let count = this as f32;

        match this {
            0..KIB => write!(f, "{:>6} B", this)?,
            KIB..MIB => write!(f, "{:>6.2} KiB", count / KIB as f32)?,
            MIB..GIB => write!(f, "{:>6.2} MiB", count / MIB as f32)?,
            GIB..TIB => write!(f, "{:>6.2} GiB", count / GIB as f32)?,
            _ => write!(f, "{:>6.2} TiB", count / TIB as f32)?,
        };

        Ok(())
    }
}
