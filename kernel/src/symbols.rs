/// Symbol table management for kernel backtrace functionality
/// 
/// This module provides symbol name resolution for instruction pointers,
/// enabling more informative backtraces with function names instead of just addresses.
/// 
/// The implementation attempts to load symbols from the kernel ELF file via limine,
/// falling back to basic placeholder symbols if that fails.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use libxernel::sync::Spinlock;
use limine::{KernelFileRequest, File};

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub address: usize,
    pub size: usize,
}

static SYMBOL_TABLE: Spinlock<BTreeMap<usize, Symbol>> = Spinlock::new(BTreeMap::new());
static KERNEL_FILE_REQUEST: KernelFileRequest = KernelFileRequest::new(0);

pub fn init() {
    // Try to load symbols from the kernel file provided by limine
    if let Some(kernel_file) = get_kernel_file() {
        load_symbols_from_kernel_file(kernel_file);
    } else {
        // Fallback: Add some example symbols for demonstration
        add_symbol("kernel_function_example".to_string(), 0x100000, 0x100);
        add_symbol("another_function".to_string(), 0x200000, 0x200);
    }
    
    crate::info!("Symbol table initialized with {} symbols", get_symbol_count());
}

fn get_kernel_file() -> Option<&'static File> {
    KERNEL_FILE_REQUEST
        .get_response()
        .get()
        .and_then(|response| response.kernel_file.as_ref())
        .map(|file| unsafe { &*file.as_ptr() })
}

fn load_symbols_from_kernel_file(kernel_file: &File) {
    // Parse the kernel ELF file to extract symbol information
    let elf_data = unsafe {
        core::slice::from_raw_parts(
            kernel_file.data.as_ptr().cast::<u8>(),
            kernel_file.size as usize,
        )
    };
    
    match elf::ElfBytes::<elf::endian::NativeEndian>::minimal_parse(elf_data) {
        Ok(elf) => {
            load_symbols_from_elf(&elf);
        }
        Err(_) => {
            crate::warning!("Failed to parse kernel ELF file for symbols");
            // Fall back to adding some basic symbols
            add_basic_symbols();
        }
    }
}

fn load_symbols_from_elf(elf: &elf::ElfBytes<elf::endian::NativeEndian>) {
    let mut symbol_count = 0;
    
    // Try to get the symbol table
    if let Ok((symtab, strtab)) = elf.symbol_table() {
        if let (Some(symbols), Some(strings)) = (symtab, strtab) {
            for symbol in symbols.iter() {
                // Only include function symbols with valid names
                if symbol.st_info & 0xf == elf::abi::STT_FUNC && symbol.st_value != 0 {
                    if let Ok(name) = strings.get(symbol.st_name as usize) {
                        if !name.is_empty() {
                            add_symbol(
                                name.to_string(),
                                symbol.st_value as usize,
                                symbol.st_size as usize,
                            );
                            symbol_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    if symbol_count == 0 {
        crate::warning!("No symbols found in kernel ELF file");
        add_basic_symbols();
    } else {
        crate::info!("Loaded {} symbols from kernel ELF", symbol_count);
    }
}

fn add_basic_symbols() {
    // Add some well-known symbols as fallback
    add_symbol("kernel_main".to_string(), 0x100000, 0x1000);
    add_symbol("panic_handler".to_string(), 0x101000, 0x200);
    add_symbol("backtrace_function".to_string(), 0x102000, 0x300);
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
        // Only return if the offset is reasonable (within symbol bounds + small padding)
        if offset < symbol.size + 1024 {
            return Some(format!("{}+0x{:x}", symbol.name, offset));
        }
    }
    
    None
}

pub fn get_symbol_count() -> usize {
    SYMBOL_TABLE.lock().len()
}

/// Get all symbols for debugging purposes
pub fn get_all_symbols() -> Vec<Symbol> {
    SYMBOL_TABLE.lock().values().cloned().collect()
}

/// Test function to verify symbol resolution is working
pub fn test_symbol_resolution() {
    crate::info!("Testing symbol resolution...");
    
    // Add a test symbol
    add_symbol("test_function".to_string(), 0x12345678, 0x100);
    
    // Test exact match
    match get_symbol(0x12345678) {
        Some(symbol) => {
            crate::info!("Exact match: {}", symbol);
            if symbol == "test_function+0x0" {
                crate::info!("✓ Exact match test passed");
            } else {
                crate::warning!("✗ Exact match test failed: expected 'test_function+0x0', got '{}'", symbol);
            }
        }
        None => crate::warning!("✗ Exact match test failed: no symbol found"),
    }
    
    // Test offset match within bounds
    match get_symbol(0x12345680) {
        Some(symbol) => {
            crate::info!("Offset match: {}", symbol);
            if symbol == "test_function+0x8" {
                crate::info!("✓ Offset match test passed");
            } else {
                crate::warning!("✗ Offset match test failed: expected 'test_function+0x8', got '{}'", symbol);
            }
        }
        None => crate::warning!("✗ Offset match test failed: no symbol found"),
    }
    
    // Test no match for far address
    if get_symbol(0x99999999).is_none() {
        crate::info!("✓ No match for unknown address (expected)");
    } else {
        crate::warning!("✗ Should not have matched unknown address");
    }
    
    crate::info!("Symbol resolution test completed");
}