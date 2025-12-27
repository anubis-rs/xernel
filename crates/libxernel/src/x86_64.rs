use crate::addr::PhysAddr;
use crate::gdt::SegmentSelector;

/// CR3 control register flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Cr3Flags: u64 {
        /// Page-level Write-Through
        const PAGE_LEVEL_WRITE_THROUGH = 1 << 3;
        /// Page-level Cache Disable
        const PAGE_LEVEL_CACHE_DISABLE = 1 << 4;
    }
}

/// CR3 control register
pub struct Cr3;

impl Cr3 {
    /// Read the current P4 table address from the CR3 register
    #[inline]
    pub fn read() -> (PhysAddr, Cr3Flags) {
        let value: u64;
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        
        let addr = PhysAddr::new(value & 0x_000f_ffff_ffff_f000);
        let flags = Cr3Flags::from_bits_truncate(value);
        (addr, flags)
    }

    /// Write a new P4 table address to the CR3 register
    #[inline]
    pub unsafe fn write(addr: PhysAddr, flags: Cr3Flags) {
        let value = addr.as_u64() | flags.bits();
        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
        }
    }
}

/// Interrupt control
pub mod interrupts {
    /// Enable interrupts
    #[inline]
    pub fn enable() {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }

    /// Disable interrupts
    #[inline]
    pub fn disable() {
        unsafe {
            core::arch::asm!("cli", options(nomem, nostack));
        }
    }

    /// Returns whether interrupts are enabled
    #[inline]
    pub fn are_enabled() -> bool {
        let rflags: u64;
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
        }
        rflags & (1 << 9) != 0
    }

    /// Execute a closure with interrupts disabled
    #[inline]
    pub fn without_interrupts<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let were_enabled = are_enabled();
        if were_enabled {
            disable();
        }
        let result = f();
        if were_enabled {
            enable();
        }
        result
    }
}

/// Segment register operations
pub trait Segment {
    /// Returns the current value of the segment register
    fn get_reg() -> SegmentSelector;
    
    /// Sets the segment register to the given selector
    unsafe fn set_reg(selector: SegmentSelector);
}

/// Code Segment register
pub struct CS;

impl Segment for CS {
    #[inline]
    fn get_reg() -> SegmentSelector {
        let segment: u16;
        unsafe {
            core::arch::asm!("mov {0:x}, cs", out(reg) segment, options(nomem, nostack, preserves_flags));
        }
        SegmentSelector::new(segment >> 3, crate::gdt::PrivilegeLevel::Ring0)
    }

    #[inline]
    unsafe fn set_reg(selector: SegmentSelector) {
        unsafe {
            core::arch::asm!(
                "push {0:r}",
                "lea {1}, [2f + rip]",
                "push {1:r}",
                "retfq",
                "2:",
                in(reg) u64::from(selector.as_u16()),
                lateout(reg) _,
                options(preserves_flags),
            );
        }
    }
}

/// Stack Segment register
pub struct SS;

impl Segment for SS {
    #[inline]
    fn get_reg() -> SegmentSelector {
        let segment: u16;
        unsafe {
            core::arch::asm!("mov {0:x}, ss", out(reg) segment, options(nomem, nostack, preserves_flags));
        }
        SegmentSelector::new(segment >> 3, crate::gdt::PrivilegeLevel::Ring0)
    }

    #[inline]
    unsafe fn set_reg(selector: SegmentSelector) {
        unsafe {
            core::arch::asm!("mov ss, {0:x}", in(reg) selector.as_u16(), options(nostack, preserves_flags));
        }
    }
}

/// Data Segment register
pub struct DS;

impl Segment for DS {
    #[inline]
    fn get_reg() -> SegmentSelector {
        let segment: u16;
        unsafe {
            core::arch::asm!("mov {0:x}, ds", out(reg) segment, options(nomem, nostack, preserves_flags));
        }
        SegmentSelector::new(segment >> 3, crate::gdt::PrivilegeLevel::Ring0)
    }

    #[inline]
    unsafe fn set_reg(selector: SegmentSelector) {
        unsafe {
            core::arch::asm!("mov ds, {0:x}", in(reg) selector.as_u16(), options(nostack, preserves_flags));
        }
    }
}

/// Extra Segment register
pub struct ES;

impl Segment for ES {
    #[inline]
    fn get_reg() -> SegmentSelector {
        let segment: u16;
        unsafe {
            core::arch::asm!("mov {0:x}, es", out(reg) segment, options(nomem, nostack, preserves_flags));
        }
        SegmentSelector::new(segment >> 3, crate::gdt::PrivilegeLevel::Ring0)
    }

    #[inline]
    unsafe fn set_reg(selector: SegmentSelector) {
        unsafe {
            core::arch::asm!("mov es, {0:x}", in(reg) selector.as_u16(), options(nostack, preserves_flags));
        }
    }
}

/// Load Task Register with the given segment selector
#[inline]
pub unsafe fn load_tss(selector: SegmentSelector) {
    unsafe {
        core::arch::asm!("ltr {0:x}", in(reg) selector.as_u16(), options(nostack, preserves_flags));
    }
}

/// Port I/O
#[derive(Debug, Clone, Copy)]
pub struct Port<T> {
    port: u16,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Port<T> {
    /// Creates a new port
    #[inline]
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl Port<u8> {
    /// Reads a byte from the port
    #[inline]
    pub unsafe fn read(&mut self) -> u8 {
        let value: u8;
        unsafe {
            core::arch::asm!("in al, dx", out("al") value, in("dx") self.port, options(nomem, nostack, preserves_flags));
        }
        value
    }

    /// Writes a byte to the port
    #[inline]
    pub unsafe fn write(&mut self, value: u8) {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") self.port, in("al") value, options(nomem, nostack, preserves_flags));
        }
    }
}

impl Port<u16> {
    /// Reads a word from the port
    #[inline]
    pub unsafe fn read(&mut self) -> u16 {
        let value: u16;
        unsafe {
            core::arch::asm!("in ax, dx", out("ax") value, in("dx") self.port, options(nomem, nostack, preserves_flags));
        }
        value
    }

    /// Writes a word to the port
    #[inline]
    pub unsafe fn write(&mut self, value: u16) {
        unsafe {
            core::arch::asm!("out dx, ax", in("dx") self.port, in("ax") value, options(nomem, nostack, preserves_flags));
        }
    }
}

impl Port<u32> {
    /// Reads a dword from the port
    #[inline]
    pub unsafe fn read(&mut self) -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("in eax, dx", out("eax") value, in("dx") self.port, options(nomem, nostack, preserves_flags));
        }
        value
    }

    /// Writes a dword to the port
    #[inline]
    pub unsafe fn write(&mut self, value: u32) {
        unsafe {
            core::arch::asm!("out dx, eax", in("dx") self.port, in("eax") value, options(nomem, nostack, preserves_flags));
        }
    }
}
