use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use libxernel::sync::Spinlock;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub address: usize,
    pub size: usize,
}

static SYMBOL_TABLE: Spinlock<BTreeMap<usize, Symbol>> = Spinlock::new(BTreeMap::new());

pub fn init() {
    // For now, just initialize an empty symbol table
    // In a full implementation, symbols would be loaded from the kernel ELF
    // or provided by the bootloader
    
    // Add some example symbols for demonstration
    add_symbol("kernel_function_example".to_string(), 0x100000, 0x100);
    add_symbol("another_function".to_string(), 0x200000, 0x200);
    
    crate::info!("Symbol table initialized with {} symbols", get_symbol_count());
}

pub fn get_symbol_count() -> usize {
    SYMBOL_TABLE.lock().len()
}

pub fn add_symbol(name: String, address: usize, size: usize) {
    let symbol = Symbol { name, address, size };
    SYMBOL_TABLE.lock().insert(address, symbol);
}

pub fn get_symbol(address: usize) -> Option<String> {
    let symbols = SYMBOL_TABLE.lock();
    
    // Find the symbol that contains this address
    // We look for the largest address that is <= our target address
    let mut best_match: Option<&Symbol> = None;
    
    for (_, symbol) in symbols.iter() {
        if symbol.address <= address && address < symbol.address + symbol.size {
            // Direct hit - address is within symbol bounds
            return Some(format!("{}+0x{:x}", symbol.name, address - symbol.address));
        } else if symbol.address <= address {
            // Potential match - keep the one with highest address <= target
            match best_match {
                None => best_match = Some(symbol),
                Some(current_best) => {
                    if symbol.address > current_best.address {
                        best_match = Some(symbol);
                    }
                }
            }
        }
    }
    
    // If we found a close match, return it with offset
    if let Some(symbol) = best_match {
        let offset = address.saturating_sub(symbol.address);
        // Only return if the offset is reasonable (within 64KB)
        if offset < 0x10000 {
            return Some(format!("{}+0x{:x}", symbol.name, offset));
        }
    }
    
    None
}

/// Get all symbols for debugging purposes
pub fn get_all_symbols() -> Vec<Symbol> {
    SYMBOL_TABLE.lock().values().cloned().collect()
}