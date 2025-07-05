use alloc::string::ToString;

pub fn init() {}

/// Log a backtrace starting from the given RBP (base pointer)
/// Now includes symbol name resolution for each frame
pub fn log_backtrace(initial_rbp: usize) {
    dbg!("==================== BACKTRACE ====================");
    
    let mut rbp = initial_rbp;
    let mut frame_count = 0;
    const MAX_FRAMES: usize = 32; // Prevent infinite loops

    while rbp != 0 && frame_count < MAX_FRAMES {
        if unsafe { *(rbp as *const usize) } == 0 {
            break;
        }

        let rip = unsafe { *(rbp as *const usize).offset(1) };

        if rip == 0 {
            break;
        }

        // Resolve symbol name for this instruction pointer
        let symbol = crate::symbols::get_symbol(rip).unwrap_or_else(|| "<unknown>".to_string());
        dbg!("#{}: 0x{:x} {}", frame_count, rip, symbol);

        rbp = unsafe { *(rbp as *const usize) };
        frame_count += 1;
    }

    if frame_count >= MAX_FRAMES {
        dbg!("... (backtrace truncated)");
    }

    dbg!("==================================================");
}

/// Get the current RBP value for backtrace purposes
#[inline(never)]
pub fn get_current_rbp() -> usize {
    let rbp: usize;
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp);
    }
    rbp
}

/// Example function to demonstrate backtrace functionality
pub fn example_backtrace() {
    let rbp = get_current_rbp();
    crate::info!("Demonstrating backtrace functionality:");
    log_backtrace(rbp);
}
