use crate::dbg;

pub fn init() {

}

// TODO: print symbol names
pub fn log_backtrace(initial_rbp: usize) {
    dbg!("==================== BACKTRACE ====================");

    let mut rbp = initial_rbp;

    while rbp != 0 {
        if unsafe { *(rbp as *const usize) } == 0 {
            break;
        }

        let rip = unsafe { *(rbp as *const usize).offset(1) };

        if rip == 0 {
            break;
        }

        let symbol = ""; // crate::symbols::get_symbol(rip);
        dbg!("0x{:x} {}", rip, symbol);

        rbp = unsafe { *(rbp as *const usize) };
    }

    dbg!("==================================================");
}
