/// Physical memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Creates a new physical address.
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Creates a new physical address, truncating bits that are not valid.
    #[inline]
    pub const fn new_truncate(addr: u64) -> Self {
        // Physical addresses on x86_64 are typically 48 bits (more conservative than 52)
        Self(addr & 0x0000_ffff_ffff_ffff)
    }

    /// Converts to a u64.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Aligns the address downwards to the given alignment.
    #[inline]
    pub const fn align_down(self, align: u64) -> Self {
        Self(align_down(self.0, align))
    }

    /// Aligns the address upwards to the given alignment.
    #[inline]
    pub const fn align_up(self, align: u64) -> Self {
        Self(align_up(self.0, align))
    }

    /// Checks whether the address is aligned to the given alignment.
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.align_down(align).0 == self.0
    }
}

impl core::ops::Add<u64> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<u64> for PhysAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl core::ops::Sub<PhysAddr> for PhysAddr {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: PhysAddr) -> Self::Output {
        self.0 - rhs.0
    }
}

/// Virtual memory address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Creates a new virtual address.
    ///
    /// # Panics
    /// Panics if the address is not canonical (bits 48-63 must be sign extension of bit 47).
    #[inline]
    pub const fn new(addr: u64) -> Self {
        assert!(
            Self::is_canonical(addr),
            "Virtual address is not canonical"
        );
        Self(addr)
    }

    /// Creates a new virtual address, truncating bits that are not valid.
    #[inline]
    pub const fn new_truncate(addr: u64) -> Self {
        Self(((addr << 16) as i64 >> 16) as u64)
    }

    /// Tries to create a new virtual address. Returns None if address is not canonical.
    #[inline]
    pub const fn try_new(addr: u64) -> Option<Self> {
        if Self::is_canonical(addr) {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Creates a virtual address from a pointer.
    #[inline]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as u64)
    }

    /// Converts to a u64.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Converts to a mutable pointer.
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Converts to a pointer.
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Aligns the address downwards to the given alignment.
    #[inline]
    pub const fn align_down(self, align: u64) -> Self {
        Self(align_down(self.0, align))
    }

    /// Aligns the address upwards to the given alignment.
    #[inline]
    pub const fn align_up(self, align: u64) -> Self {
        Self(align_up(self.0, align))
    }

    /// Checks whether the address is aligned to the given alignment.
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.align_down(align).0 == self.0
    }

    /// Checks if the address is canonical (bits 48-63 are sign extension of bit 47).
    #[inline]
    const fn is_canonical(addr: u64) -> bool {
        match addr >> 47 {
            0 | 0x1ffff => true,
            _ => false,
        }
    }

    /// Returns the 12-bit page offset for 4KiB pages
    #[inline]
    pub const fn page_offset(self) -> u16 {
        (self.0 & 0xfff) as u16
    }

    /// Returns the 9-bit level 1 page table index
    #[inline]
    pub const fn p1_index(self) -> crate::paging::PageTableIndex {
        crate::paging::PageTableIndex::new_truncate(((self.0 >> 12) & 0x1ff) as u16)
    }

    /// Returns the 9-bit level 2 page table index
    #[inline]
    pub const fn p2_index(self) -> crate::paging::PageTableIndex {
        crate::paging::PageTableIndex::new_truncate(((self.0 >> 21) & 0x1ff) as u16)
    }

    /// Returns the 9-bit level 3 page table index
    #[inline]
    pub const fn p3_index(self) -> crate::paging::PageTableIndex {
        crate::paging::PageTableIndex::new_truncate(((self.0 >> 30) & 0x1ff) as u16)
    }

    /// Returns the 9-bit level 4 page table index
    #[inline]
    pub const fn p4_index(self) -> crate::paging::PageTableIndex {
        crate::paging::PageTableIndex::new_truncate(((self.0 >> 39) & 0x1ff) as u16)
    }
}

impl core::ops::Add<u64> for VirtAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl core::ops::Sub<u64> for VirtAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl core::ops::Sub<VirtAddr> for VirtAddr {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: VirtAddr) -> Self::Output {
        self.0 - rhs.0
    }
}

impl core::ops::AddAssign<u64> for VirtAddr {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl core::ops::SubAssign<u64> for VirtAddr {
    #[inline]
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl core::fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl core::fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl core::fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl core::fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}

/// Align downwards. Returns the greatest value less than or equal to `addr` that is a multiple of `align`.
#[inline]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Align upwards. Returns the smallest value greater than or equal to `addr` that is a multiple of `align`.
#[inline]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    let mask = align - 1;
    if addr & mask == 0 {
        addr
    } else {
        (addr | mask) + 1
    }
}
