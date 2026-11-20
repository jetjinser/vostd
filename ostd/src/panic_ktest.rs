// SPDX-License-Identifier: MPL-2.0
//! Minimal abort path for the single-BSP ktest kernel.

/// Stops execution after an unrecoverable allocation or runtime failure.
pub fn abort() -> ! {
    loop {
        // SAFETY: This is the terminal panic/abort path for the kernel.
        unsafe { core::arch::asm!("cli", "hlt", options(nomem, nostack)) };
    }
}
