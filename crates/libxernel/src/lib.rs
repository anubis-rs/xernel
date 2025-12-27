#![no_std]
#![allow(unused)]

pub mod boot;
pub mod collections;
pub mod on_drop;
pub mod sync;
pub mod syscall;

#[cfg(feature = "kernel")]
pub mod ipl;

#[cfg(feature = "kernel")]
pub mod addr;
#[cfg(feature = "kernel")]
pub mod paging;
#[cfg(feature = "kernel")]
pub mod gdt;
#[cfg(feature = "kernel")]
pub mod x86_64;
