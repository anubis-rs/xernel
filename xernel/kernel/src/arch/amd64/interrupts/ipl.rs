use core::arch::asm;

use crate::{arch::amd64::{apic::APIC, interrupts::dpc::DPC_VECTOR}, cpu::current_cpu};

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

impl From<u64> for IPL {
    fn from(value: u64) -> Self {
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

impl From<u8> for IPL {
    fn from(value: u8) -> Self {
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

pub fn get_spl() -> IPL {
    let ipl: u64;

    unsafe {
        asm!("mov {}, cr8", out(reg) ipl, options(nomem, nostack, preserves_flags));
    }

    debug!("get_spl {:?}", IPL::from(ipl));

    IPL::from(ipl)
}

pub fn set_ipl(ipl: IPL) -> IPL {
    let requested_ipl = ipl as u64;
    let old_ipl = get_spl() as u64;

    println!("set_ipl requested_ipl: {:?} old_ipl: {:?}", requested_ipl, old_ipl);

    unsafe {
        asm!("mov cr8, {}", in(reg) requested_ipl, options(nomem, nostack, preserves_flags));
    }

    if old_ipl > requested_ipl {
        ipl_lowered(IPL::from(old_ipl), IPL::from(requested_ipl));
    }

    IPL::from(old_ipl)
}

pub fn raise_spl(spl: IPL) -> IPL {
    debug!("raise_spl with spl {:?}", spl);
    let old_ipl = get_spl();

    assert!(old_ipl as u64 <= spl as u64);

    if old_ipl < spl {
        set_ipl(spl);
    }

    old_ipl
}

pub fn ipl_lowered(from: IPL, to: IPL) {

    debug!("IPL lowered from {:?} to {:?}", from, to);

    if (to as u8) < (IPL::IPLDPC as u8) && current_cpu().dpc_queue.read().dpcs.len() > 0 {
        APIC.send_ipi(current_cpu().lapic_id, *DPC_VECTOR as u32);
        current_cpu().dpc_queue.write().work_off();
    }

}
