use core::arch::asm;

#[derive(Clone, Copy)]
pub enum IPL {
    IPL0 = 0,
    IPLAPC = 1,
    IPLDPC = 2,
    IPLDevice = 13,
    IPLClock = 14,
    IPLHigh = 15
}

impl From<u64> for IPL {
    fn from(value: u64) -> Self {
        match value {
            0 => IPL::IPL0,
            1 => IPL::IPLAPC,
            2 => IPL::IPLDPC,
            13 => IPL::IPLDevice,
            14 => IPL::IPLClock,
            15 => IPL::IPLDevice,
            _ => panic!("Bad IPL"),
        }
    }
}

pub fn get_spl() -> IPL {
    let ipl: u64;

    unsafe {
        asm!("mov {}, cr8", out(reg) ipl, options(nomem, nostack, preserves_flags));
    }

    IPL::from(ipl)
}

pub fn set_ipl(ipl: IPL) -> IPL {
    let requested_ipl = ipl as u64;
    let old_ipl = get_spl() as u64;

    unsafe {
        asm!("mov cr8, {}", in(reg) requested_ipl, options(nomem, nostack, preserves_flags));
    }

    if old_ipl > requested_ipl {
        ipl_lowered(IPL::from(old_ipl), IPL::from(requested_ipl));
    }

    IPL::from(old_ipl)
}

pub fn raise_spl(spl: IPL) -> IPL {
    let old_ipl = get_spl();

    assert!(old_ipl as u64 <= spl as u64);

    if old_ipl < spl {
        set_ipl(spl)
    }

    old_ipl
}

pub fn ipl_lowered(from: IPL, to: IPL) {
    // Dispatch DPC self ipi here if ipl went below IPLDPC
}