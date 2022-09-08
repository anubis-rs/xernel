use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let idt = InterruptDescriptorTable::new();
        idt
    };
}

pub fn init() {
    IDT.load();
}
