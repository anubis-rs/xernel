// Credits to Stupremee (https://github.com/Stupremee)
// https://github.com/Stupremee/novos/blob/main/crates/kernel/src/allocator.rs

pub mod buddy;
pub mod unit;

use core::fmt;

/// Result for every memory allocation operation.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Aligns the given `addr` upwards to `align`.
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Any error that can happen while allocating or deallocating memory.
#[derive(Debug)]
pub enum Error {
    /// region was too small
    RegionTooSmall,
    /// the `end` pointer of a memory region was before the `start` pointer
    InvalidRegion,
    /// order exceeded the maximum order
    OrderTooLarge,
    /// no free memory left
    NoMemoryAvailable,
    /// can't allocate zero pages
    AllocateZeroPages,
    /// this is not a real error and should never be thrown somewhere
    NoSlabForLayout,
    /// `NonNull` was null
    ///
    /// Mostly just a safety mechanism to avoid UB.
    NullPointer,
}

/// Statistics for a memory allocator.
#[derive(Debug, Clone)]
pub struct AllocStats {
    /// The name of the allocator that collected these stat.s
    pub name: &'static str,
    /// The number of size that were allocated.
    pub allocated: usize,
    /// The number of bytes that are left for allocation.
    pub free: usize,
    /// The total number of bytes that this allocator has available for allocation.
    pub total: usize,
}

impl AllocStats {
    /// Create a new [`AllocStats`] instance for the given allocator name.
    pub const fn with_name(name: &'static str) -> Self {
        Self {
            name,
            free: 0,
            allocated: 0,
            total: 0,
        }
    }
}

impl fmt::Display for AllocStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        self.name.chars().try_for_each(|_| write!(f, "-"))?;

        writeln!(f, "\n{:<11} {}", "Allocated:", unit::bytes(self.allocated))?;
        writeln!(f, "{:<11} {}", "Free:", unit::bytes(self.free))?;
        writeln!(f, "{:<11} {}", "Total:", unit::bytes(self.total))?;

        self.name.chars().try_for_each(|_| write!(f, "-"))?;
        Ok(())
    }
}
