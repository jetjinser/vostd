// SPDX-License-Identifier: MPL-2.0
//! Minimal COM1 support for the BSP ktest path.

unsafe fn outb(port: u16, value: u8) {
    // SAFETY: The ktest kernel reserves and exclusively uses COM1.
    unsafe { core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags)) }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    // SAFETY: The ktest kernel reserves and exclusively uses COM1.
    unsafe { core::arch::asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags)) };
    value
}

/// Initializes the serial port.
pub(crate) fn init() {
    unsafe {
        outb(0x3f9, 0x00);
        outb(0x3fb, 0x80);
        outb(0x3f8, 0x03);
        outb(0x3f9, 0x00);
        outb(0x3fb, 0x03);
        outb(0x3fa, 0xc7);
        outb(0x3fc, 0x0b);
    }
}

/// Sends a byte on the serial port.
pub(crate) fn send(data: u8) {
    unsafe {
        while inb(0x3fd) & 0x20 == 0 {}
        outb(0x3f8, data);
    }
}
