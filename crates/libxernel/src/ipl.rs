use core::arch::asm;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
pub enum IPL {
    Passive = 0,
    APC = 1,
    DPC = 2,
    Device = 13,
    Clock = 14,
    High = 15,
}

impl From<usize> for IPL {
    fn from(value: usize) -> Self {
        match value {
            0 => IPL::Passive,
            1 => IPL::APC,
            2 => IPL::DPC,
            13 => IPL::Device,
            14 => IPL::Clock,
            15 => IPL::High,
            _ => panic!("Bad IPL"),
        }
    }
}

impl From<u64> for IPL {
    fn from(value: u64) -> Self {
        IPL::from(value as usize)
    }
}

impl From<u8> for IPL {
    fn from(value: u8) -> Self {
        IPL::from(value as usize)
    }
}

fn set_ipl(ipl: IPL) {
    unsafe {
        asm!("mov cr8, {}", in(reg) ipl as u64, options(nomem, nostack, preserves_flags));
    }
}

pub fn get_ipl() -> IPL {
    let ipl: u64;

    unsafe {
        asm!("mov {}, cr8", out(reg) ipl, options(nomem, nostack, preserves_flags));
    }

    IPL::from(ipl)
}

pub fn raise_ipl(ipl: IPL) -> IPL {
    let old_ipl = get_ipl();

    assert!(old_ipl as u64 <= ipl as u64);

    set_ipl(ipl);

    old_ipl
}

pub fn splx(ipl: IPL) {
    assert!(ipl as u64 <= get_ipl() as u64);

    set_ipl(ipl);
}
