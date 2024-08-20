use core::arch::asm;

use crate::cpu::current_cpu;
use crate::dpc::dpc_interrupt_dispatch;

#[macro_export]
macro_rules! lock_with_ipl {
    ($name:ident) => {{
        let old = raise_ipl(IPL::DPC);
        OnDrop::new($name.lock(), move || {
            set_ipl(old);
        })
    }};
    ($name:ident, $ipl:expr) => {{
        let _ = raise_ipl(IPL::DPC);
        OnDrop::new($name.lock(), || {
            set_ipl($ipl);
        })
    }};
}

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

pub fn get_ipl() -> IPL {
    let ipl: u64;

    unsafe {
        asm!("mov {}, cr8", out(reg) ipl, options(nomem, nostack, preserves_flags));
    }

    IPL::from(ipl)
}

pub fn set_ipl(ipl: IPL) -> IPL {
    let requested_ipl = ipl as u64;
    let old_ipl = get_ipl() as u64;

    unsafe {
        asm!("mov cr8, {}", in(reg) requested_ipl, options(nomem, nostack, preserves_flags));
    }

    if old_ipl > requested_ipl {
        ipl_lowered(IPL::from(old_ipl), IPL::from(requested_ipl));
    }

    IPL::from(old_ipl)
}

pub fn raise_ipl(ipl: IPL) -> IPL {
    let old_ipl = get_ipl();

    assert!(old_ipl as u64 <= ipl as u64);

    if old_ipl < ipl {
        set_ipl(ipl);
    }

    old_ipl
}

pub fn ipl_lowered(_from: IPL, to: IPL) {
    if (to as u8) < (IPL::DPC as u8) && !current_cpu().dpc_queue.read().dpcs.is_empty() {
        dpc_interrupt_dispatch();
    }
}
