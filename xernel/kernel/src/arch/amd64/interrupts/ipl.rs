use core::arch::asm;

use crate::cpu::current_cpu;
use crate::dpc::dpc_interrupt_dispatch;

#[macro_export]
macro_rules! lock_with_ipl {
    ($name:ident) => {
        {
            let old = raise_ipl(IPL::IPLDPC);
            OnDrop::new($name.lock(), move || { set_ipl(old); })
        }
    };
    ($name:ident, $ipl:expr) => {
        {
            let _ = raise_ipl(IPL::IPLDPC);
            OnDrop::new($name.lock(), || { set_ipl($ipl); })
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum IPL {
    IPL0 = 0,
    IPLAPC = 1,
    IPLDPC = 2,
    IPLDevice = 13,
    IPLClock = 14,
    IPLHigh = 15,
}

impl From<usize> for IPL {
    fn from(value: usize) -> Self {
        match value {
            0 => IPL::IPL0,
            1 => IPL::IPLAPC,
            2 => IPL::IPLDPC,
            13 => IPL::IPLDevice,
            14 => IPL::IPLClock,
            15 => IPL::IPLHigh,
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
    if (to as u8) < (IPL::IPLDPC as u8) && current_cpu().dpc_queue.read().dpcs.len() > 0 {
        dpc_interrupt_dispatch();
    }
}
