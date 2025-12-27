use crate::addr::{PhysAddr, VirtAddr};
use core::fmt;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

bitflags::bitflags! {
    /// Page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageTableFlags: u64 {
        /// Present; must be 1 to map a page
        const PRESENT = 1 << 0;
        /// Writable; if 0, writes may not be allowed
        const WRITABLE = 1 << 1;
        /// User/Supervisor; if 0, user-mode accesses are not allowed
        const USER_ACCESSIBLE = 1 << 2;
        /// Page-level write-through
        const WRITE_THROUGH = 1 << 3;
        /// Page-level cache disable
        const NO_CACHE = 1 << 4;
        /// Accessed; indicates whether software has accessed the page
        const ACCESSED = 1 << 5;
        /// Dirty; indicates whether software has written to the page
        const DIRTY = 1 << 6;
        /// Huge page
        const HUGE_PAGE = 1 << 7;
        /// Global; if CR4.PGE = 1, determines whether the translation is global
        const GLOBAL = 1 << 8;
        /// Available to OS
        const BIT_9 = 1 << 9;
        /// Available to OS
        const BIT_10 = 1 << 10;
        /// Available to OS
        const BIT_11 = 1 << 11;
        /// Available to OS
        const BIT_52 = 1 << 52;
        /// Available to OS
        const BIT_53 = 1 << 53;
        /// Available to OS
        const BIT_54 = 1 << 54;
        /// Available to OS
        const BIT_55 = 1 << 55;
        /// Available to OS
        const BIT_56 = 1 << 56;
        /// Available to OS
        const BIT_57 = 1 << 57;
        /// Available to OS
        const BIT_58 = 1 << 58;
        // Bits 59-62 are RESERVED and must be 0
        /// No execute; if 1, instruction fetches are not allowed
        const NO_EXECUTE = 1 << 63;
    }
}

/// A page table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Creates a new unused page table entry
    #[inline]
    pub const fn new() -> Self {
        Self { entry: 0 }
    }

    /// Returns whether this entry is zero
    #[inline]
    pub const fn is_unused(&self) -> bool {
        self.entry == 0
    }

    /// Sets this entry to zero
    #[inline]
    pub fn set_unused(&mut self) {
        self.entry = 0;
    }

    /// Returns the flags of this entry
    #[inline]
    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.entry)
    }

    /// Returns the physical address mapped by this entry
    #[inline]
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.entry & 0x000f_ffff_ffff_f000)
    }

    /// Returns the physical frame mapped by this entry
    #[inline]
    pub fn frame<S: PageSize>(&self) -> Option<PhysFrame<S>> {
        if !self.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(PhysFrame::containing_address(self.addr()))
    }

    /// Sets the entry
    #[inline]
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        self.entry = (addr.as_u64() & 0x000f_ffff_ffff_f000) | flags.bits();
    }

    /// Sets the flags
    #[inline]
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.entry = self.addr().as_u64() | flags.bits();
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.finish()
    }
}

/// The number of entries in a page table
const ENTRY_COUNT: usize = 512;

/// A page table
#[repr(align(4096))]
#[repr(C)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    /// Creates a new empty page table
    #[inline]
    pub const fn new() -> Self {
        const EMPTY: PageTableEntry = PageTableEntry::new();
        Self {
            entries: [EMPTY; ENTRY_COUNT],
        }
    }

    /// Zeroes all entries
    #[inline]
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }

    /// Returns an iterator over the entries
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    /// Returns a mutable iterator over the entries
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: PageTableIndex) -> &Self::Output {
        &self.entries[usize::from(index.0)]
    }
}

impl IndexMut<PageTableIndex> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: PageTableIndex) -> &mut Self::Output {
        &mut self.entries[usize::from(index.0)]
    }
}

impl fmt::Debug for PageTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PageTable").finish()
    }
}

/// An index into a page table
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// Creates a new index from a u16. Panics if the value is >= 512.
    #[inline]
    pub const fn new(index: u16) -> Self {
        assert!(index < 512, "PageTableIndex out of range");
        Self(index)
    }

    /// Creates a new index from a u16, truncating to 9 bits.
    #[inline]
    pub const fn new_truncate(index: u16) -> Self {
        Self(index % 512)
    }
}

impl From<PageTableIndex> for u16 {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        index.0
    }
}

impl From<PageTableIndex> for u32 {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        index.0 as u32
    }
}

impl From<PageTableIndex> for u64 {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        index.0 as u64
    }
}

/// Trait for page sizes
pub trait PageSize: Copy + Eq + PartialOrd + Ord {
    /// The size in bytes
    const SIZE: u64;

    /// The size as usize
    const SIZE_AS_USIZE: usize = Self::SIZE as usize;
}

/// A 4KiB page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size4KiB {}

impl PageSize for Size4KiB {
    const SIZE: u64 = 4096;
}

/// A 2MiB page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size2MiB {}

impl PageSize for Size2MiB {
    const SIZE: u64 = 2 * 1024 * 1024;
}

/// A 1GiB page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size1GiB {}

impl PageSize for Size1GiB {
    const SIZE: u64 = 1024 * 1024 * 1024;
}

/// A virtual memory page
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct Page<S: PageSize = Size4KiB> {
    start_address: VirtAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> Page<S> {
    /// Returns the page that contains the given virtual address
    #[inline]
    pub const fn containing_address(address: VirtAddr) -> Self {
        Self {
            start_address: VirtAddr::new(address.as_u64() & !(S::SIZE - 1)),
            size: PhantomData,
        }
    }

    /// Creates a page from a start address. Returns None if the address is not page-aligned.
    #[inline]
    pub const fn from_start_address(address: VirtAddr) -> Result<Self, ()> {
        if address.as_u64() % S::SIZE != 0 {
            return Err(());
        }
        Ok(Self {
            start_address: address,
            size: PhantomData,
        })
    }

    /// Returns the start address of the page
    #[inline]
    pub const fn start_address(self) -> VirtAddr {
        self.start_address
    }

    /// Returns the size of the page
    #[inline]
    pub const fn size(self) -> u64 {
        S::SIZE
    }

    /// Returns the page table index for the given level (P4=4, P3=3, P2=2, P1=1)
    #[inline]
    pub const fn page_table_index(self, level: u8) -> PageTableIndex {
        let addr = self.start_address.as_u64();
        let shift = 12 + (level as u64 - 1) * 9;
        PageTableIndex::new_truncate(((addr >> shift) & 0x1ff) as u16)
    }

    /// Returns the P4 (level 4) page table index
    #[inline]
    pub const fn p4_index(self) -> PageTableIndex {
        self.page_table_index(4)
    }

    /// Returns the P3 (level 3) page table index
    #[inline]
    pub const fn p3_index(self) -> PageTableIndex {
        self.page_table_index(3)
    }

    /// Returns the P2 (level 2) page table index
    #[inline]
    pub const fn p2_index(self) -> PageTableIndex {
        self.page_table_index(2)
    }

    /// Returns the P1 (level 1) page table index
    #[inline]
    pub const fn p1_index(self) -> PageTableIndex {
        self.page_table_index(1)
    }
}

impl<S: PageSize> fmt::Debug for Page<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Page")
            .field("start_address", &self.start_address)
            .field("size", &S::SIZE)
            .finish()
    }
}

impl<S: PageSize> core::ops::Add<u64> for Page<S> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self::containing_address(self.start_address + rhs * S::SIZE)
    }
}

impl<S: PageSize> core::ops::Sub<u64> for Page<S> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self::containing_address(self.start_address - rhs * S::SIZE)
    }
}

/// A physical memory frame
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct PhysFrame<S: PageSize = Size4KiB> {
    start_address: PhysAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> PhysFrame<S> {
    /// Returns the frame that contains the given physical address
    #[inline]
    pub const fn containing_address(address: PhysAddr) -> Self {
        Self {
            start_address: PhysAddr::new(address.as_u64() & !(S::SIZE - 1)),
            size: PhantomData,
        }
    }

    /// Creates a frame from a start address. Returns None if the address is not frame-aligned.
    #[inline]
    pub const fn from_start_address(address: PhysAddr) -> Result<Self, ()> {
        if address.as_u64() % S::SIZE != 0 {
            return Err(());
        }
        Ok(Self {
            start_address: address,
            size: PhantomData,
        })
    }

    /// Returns the start address of the frame
    #[inline]
    pub const fn start_address(self) -> PhysAddr {
        self.start_address
    }

    /// Returns the size of the frame
    #[inline]
    pub const fn size(self) -> u64 {
        S::SIZE
    }
}

impl<S: PageSize> fmt::Debug for PhysFrame<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PhysFrame")
            .field("start_address", &self.start_address)
            .field("size", &S::SIZE)
            .finish()
    }
}

impl<S: PageSize> core::ops::Add<u64> for PhysFrame<S> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self::containing_address(self.start_address + rhs * S::SIZE)
    }
}

impl<S: PageSize> core::ops::Sub<u64> for PhysFrame<S> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self::containing_address(self.start_address - rhs * S::SIZE)
    }
}
