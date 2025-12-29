use core::arch::asm;

#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(preserves_flags, nomem, nostack)
        );
    }
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") ret,
            options(preserves_flags, nomem, nostack)
        );
    }

    ret
}

#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(preserves_flags, nomem, nostack)
        );
    }
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    unsafe {
        asm!(
            "in ax, dx",
            out("ax") ret,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
    }

    ret
}

#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(preserves_flags, nomem, nostack)
        );
    }
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let ret: u32;

    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") ret,
            options(nomem, nostack, preserves_flags)
        );
    }

    ret
}
