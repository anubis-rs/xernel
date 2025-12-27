use core::mem;

bitflags::bitflags! {
    /// Flags for a GDT descriptor
    #[derive(Debug, Clone, Copy)]
    pub struct DescriptorFlags: u64 {
        const ACCESSED = 1 << 40;
        const WRITABLE = 1 << 41;
        const CONFORMING = 1 << 42;
        const EXECUTABLE = 1 << 43;
        const USER_SEGMENT = 1 << 44;
        const DPL_RING_3 = 3 << 45;
        const PRESENT = 1 << 47;
        const AVAILABLE = 1 << 52;
        const LONG_MODE = 1 << 53;
        const DEFAULT_SIZE = 1 << 54;
        const GRANULARITY = 1 << 55;
        
        const COMMON = Self::USER_SEGMENT.bits() 
            | Self::PRESENT.bits() 
            | Self::WRITABLE.bits()
            | Self::ACCESSED.bits()
            | Self::GRANULARITY.bits();
            
        const KERNEL_CODE64 = Self::COMMON.bits()
            | Self::EXECUTABLE.bits()
            | Self::LONG_MODE.bits();
            
        const KERNEL_DATA = Self::COMMON.bits();
        
        const USER_CODE64 = Self::KERNEL_CODE64.bits() | Self::DPL_RING_3.bits();
        const USER_DATA = Self::KERNEL_DATA.bits() | Self::DPL_RING_3.bits();
    }
}

/// A 64-bit segment descriptor
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Descriptor(u64);

impl Descriptor {
    /// Creates a kernel code segment descriptor
    pub const fn kernel_code_segment() -> Descriptor {
        Descriptor(DescriptorFlags::KERNEL_CODE64.bits())
    }

    /// Creates a kernel data segment descriptor
    pub const fn kernel_data_segment() -> Descriptor {
        Descriptor(DescriptorFlags::KERNEL_DATA.bits())
    }

    /// Creates a user code segment descriptor
    pub const fn user_code_segment() -> Descriptor {
        Descriptor(DescriptorFlags::USER_CODE64.bits())
    }

    /// Creates a user data segment descriptor
    pub const fn user_data_segment() -> Descriptor {
        Descriptor(DescriptorFlags::USER_DATA.bits())
    }

    /// Creates a TSS system segment descriptor
    pub fn tss_segment(tss: &'static TaskStateSegment) -> Descriptor {
        let ptr = tss as *const _ as u64;

        let mut low = DescriptorFlags::PRESENT.bits();
        // base
        low |= ((ptr & 0xff_ffff) << 16) | (((ptr >> 24) & 0xff) << 56);
        // limit (sizeof(TaskStateSegment) - 1)
        let limit = (mem::size_of::<TaskStateSegment>() - 1) as u64;
        low |= limit & 0xffff;
        low |= ((limit >> 16) & 0xf) << 48;
        // type: 64-bit TSS (Available)
        low |= 0x9 << 40;

        Descriptor(low)
    }

    /// Returns the raw u64 value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// The upper 8 bytes of a TSS descriptor
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct DescriptorUpper(u64);

impl DescriptorUpper {
    fn from_tss(tss: &'static TaskStateSegment) -> Self {
        let ptr = tss as *const _ as u64;
        DescriptorUpper(ptr >> 32)
    }
}

/// Global Descriptor Table
pub struct GlobalDescriptorTable {
    table: [u64; 8],
    len: usize,
}

impl GlobalDescriptorTable {
    /// Creates a new GDT with a null descriptor
    pub const fn new() -> Self {
        Self {
            table: [0; 8],
            len: 1,
        }
    }

    /// Appends a descriptor to the GDT
    pub fn append(&mut self, descriptor: Descriptor) -> SegmentSelector {
        let index = self.len;
        
        // Check for TSS descriptor (they take 2 entries)
        let is_tss = (descriptor.0 >> 40) & 0xf == 0x9;
        
        if is_tss {
            if index + 1 >= self.table.len() {
                panic!("GDT is full");
            }
            self.table[index] = descriptor.0;
            // Upper 8 bytes of TSS descriptor
            self.table[index + 1] = descriptor.0 >> 32;
            self.len += 2;
        } else {
            if index >= self.table.len() {
                panic!("GDT is full");
            }
            self.table[index] = descriptor.0;
            self.len += 1;
        }

        SegmentSelector::new(index as u16, PrivilegeLevel::Ring0)
    }

    /// Loads the GDT
    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (self.len * mem::size_of::<u64>() - 1) as u16,
            base: self.table.as_ptr() as u64,
        };

        unsafe {
            load_gdt(&ptr);
        }
    }
}

impl core::fmt::Debug for GlobalDescriptorTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GlobalDescriptorTable")
            .field("len", &self.len)
            .finish()
    }
}

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[inline]
unsafe fn load_gdt(gdt: *const DescriptorTablePointer) {
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) gdt, options(readonly, nostack, preserves_flags));
    }
}

/// A segment selector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    /// Creates a new segment selector
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self(index << 3 | (rpl as u16))
    }

    /// Returns the index
    pub const fn index(self) -> u16 {
        self.0 >> 3
    }

    /// Returns the raw u16 value
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// CPU privilege levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

/// Task State Segment
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    /// Privilege Stack Table
    pub privilege_stack_table: [u64; 3],
    reserved_2: u64,
    /// Interrupt Stack Table
    pub interrupt_stack_table: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    /// I/O Map Base Address
    pub iomap_base: u16,
}

impl TaskStateSegment {
    /// Creates a new TSS with all values set to zero
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            privilege_stack_table: [0; 3],
            reserved_2: 0,
            interrupt_stack_table: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: 0,
        }
    }
}
