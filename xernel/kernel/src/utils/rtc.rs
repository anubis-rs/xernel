use crate::{arch::amd64::ports::{inb, outb}, println};
use core::arch::asm;
const CMOSAddress: u16 = 0x70;
const CMOSData: u16 = 0x71;

pub struct Rtc;

impl Rtc {
    pub fn read() {
        
        let status: u8 = Rtc::read_cmos(0x0b);

        let bcd: bool = !(status & 0x04) > 0;

        while Rtc::read_cmos(0x0A) & 0x80 > 0 {
            unsafe {
                asm!("pause");
            }
        }

        let second = Rtc::decode(Rtc::read_cmos(0x0), bcd);
        let minute = Rtc::decode(Rtc::read_cmos(0x02), bcd);
        let hour = Rtc::decode(Rtc::read_cmos(0x04), bcd);
        let day = Rtc::decode(Rtc::read_cmos(0x07), bcd);
        let month = Rtc::decode(Rtc::read_cmos(0x08), bcd);
        let year = Rtc::decode(Rtc::read_cmos(0x09), bcd) + 2000;

        println!("Booted at: {}-{}-{} {}:{}:{} GMT", year, month, day, hour, minute, second);

    }

    fn decode(value: u8, bcd: bool) -> i64 {
        if bcd {
            ((value & 0x0f) + ((value / 16) * 10)).into()
        } else {
            value as i64
        }
    }

    fn read_cmos(reg: u8) -> u8 {
        unsafe {
            outb(CMOSAddress, reg);
            return inb(CMOSData);
        }
    }
}
