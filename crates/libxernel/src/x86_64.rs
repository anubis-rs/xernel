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

/// Page fault error code
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PageFaultErrorCode: u64 {
        /// If set, the page fault was caused by a page-protection violation.
        /// If not set, the page fault was caused by a non-present page.
        const PROTECTION_VIOLATION = 1 << 0;
        /// If set, the access causing the fault was a write.
        const CAUSED_BY_WRITE = 1 << 1;
        /// If set, the access causing the fault originated in user mode.
        const USER_MODE = 1 << 2;
        /// If set, the fault was caused by a reserved bit violation.
        const MALFORMED_TABLE = 1 << 3;
        /// If set, the fault was caused by an instruction fetch.
        const INSTRUCTION_FETCH = 1 << 4;
    }
}

/// RFLAGS register
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct RFlags: u64 {
        const INTERRUPT_FLAG = 1 << 9;
    }
}

/// Extended Feature Enable Register flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EferFlags: u64 {
        const SYSTEM_CALL_EXTENSIONS = 1 << 0;
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

    /// Read the current P4 table address from the CR3 register (raw u64 value)
    #[inline]
    pub fn read_raw() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        value
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

/// Model Specific Registers

/// EFER - Extended Feature Enable Register
pub struct Efer;

impl Efer {
    const MSR: u32 = 0xC000_0080;

    /// Read the current EFER flags
    #[inline]
    pub fn read() -> EferFlags {
        let low: u32;
        let high: u32;
        unsafe {
            core::arch::asm!(
                "rdmsr",
                in("ecx") Self::MSR,
                out("eax") low,
                out("edx") high,
                options(nomem, nostack, preserves_flags),
            );
        }
        let value = (high as u64) << 32 | (low as u64);
        EferFlags::from_bits_truncate(value)
    }

    /// Write EFER flags
    #[inline]
    pub unsafe fn write(flags: EferFlags) {
        let value = flags.bits();
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") Self::MSR,
                in("eax") value as u32,
                in("edx") (value >> 32) as u32,
                options(nomem, nostack, preserves_flags),
            );
        }
    }
}

/// STAR - System Call Target Address Register
pub struct Star;

impl Star {
    const MSR: u32 = 0xC000_0081;

    /// Write STAR MSR for syscall/sysret
    #[inline]
    pub fn write(
        user_code_selector: SegmentSelector,
        user_data_selector: SegmentSelector,
        kernel_code_selector: SegmentSelector,
        kernel_data_selector: SegmentSelector,
    ) -> Result<(), ()> {
        // STAR layout:
        // Bits 63:48 - User CS (sysret) and SS (sysret+8)
        // Bits 47:32 - Kernel CS (syscall) and SS (syscall+8)
        let user_base = user_code_selector.as_u16() as u64;
        let kernel_base = kernel_code_selector.as_u16() as u64;
        let value = (user_base << 48) | (kernel_base << 32);
        
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") Self::MSR,
                in("eax") value as u32,
                in("edx") (value >> 32) as u32,
                options(nomem, nostack, preserves_flags),
            );
        }
        Ok(())
    }
}

/// LSTAR - Long Mode System Call Target Address Register
pub struct LStar;

impl LStar {
    const MSR: u32 = 0xC000_0082;

    /// Write LSTAR MSR
    #[inline]
    pub unsafe fn write(addr: crate::addr::VirtAddr) {
        let value = addr.as_u64();
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") Self::MSR,
                in("eax") value as u32,
                in("edx") (value >> 32) as u32,
                options(nomem, nostack, preserves_flags),
            );
        }
    }
}

/// SFMASK - System Call Flag Mask Register
pub struct SFMask;

impl SFMask {
    const MSR: u32 = 0xC000_0084;

    /// Write SFMASK MSR
    #[inline]
    pub unsafe fn write(flags: RFlags) {
        let value = flags.bits();
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") Self::MSR,
                in("eax") value as u32,
                in("edx") (value >> 32) as u32,
                options(nomem, nostack, preserves_flags),
            );
        }
    }
}
