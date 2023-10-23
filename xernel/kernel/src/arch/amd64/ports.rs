use x86_64::instructions::port::Port;

pub fn outb(port: u16, value: u8) {
    let mut port = Port::new(port);

    unsafe {
        port.write(value);
    }
}

pub fn inb(port: u16) -> u8 {
    let mut port = Port::new(port);

    unsafe { port.read() }
}
