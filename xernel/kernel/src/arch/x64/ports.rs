use x86_64::instructions::port::Port;

pub fn outb(port: u16, value: u8) {
    let mut port = Port::new(port);

    unsafe { port.write(value); }
}