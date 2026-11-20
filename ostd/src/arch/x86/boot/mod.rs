// SPDX-License-Identifier: MPL-2.0
//! The x86 boot module defines the entrypoints of Asterinas and
//! the corresponding headers for different x86 boot protocols.
//!
//! We directly support
//!
//!  - Multiboot
//!  - Multiboot2
//!  - Linux x86 Boot Protocol
//!
//! without any additional configurations.
//!
//! Asterinas diffrentiates the boot protocol by the entry point
//! chosen by the boot loader. In each entry point function,
//! the universal callback registration method from
//! `crate::boot` will be called. Thus the initialization of
//! boot information is transparent for the upper level kernel.
//!
#[cfg(not(ktest))]
mod linux_boot;
#[cfg(not(ktest))]
mod multiboot;
#[cfg(ktest)]
#[verifier::external]
mod multiboot;
mod multiboot2;

#[cfg(not(ktest))]
pub mod smp;
#[cfg(ktest)]
pub(crate) mod smp {
    use crate::boot::memory_region::{MemoryRegion, MemoryRegionType};

    pub(super) fn reclaimable_memory_region() -> MemoryRegion {
        MemoryRegion::new(0, 0, MemoryRegionType::Reclaimable)
    }
}

use core::arch::global_asm;

global_asm!(
    include_str!("bsp_boot.S"),
    KCODE64 = const 0x00AF_9B00_0000_FFFFu64,
    KDATA = const 0x00CF_9300_0000_FFFFu64,
    KCODE32 = const 0x00CF_9B00_0000_FFFFu64,
);
global_asm!(include_str!("ap_boot.S"));

#[cfg(ktest)]
#[no_mangle]
unsafe extern "sysv64" fn __linux_boot(_: u64) -> ! {
    halt_boot_cpu()
}

#[cfg(ktest)]
#[no_mangle]
extern "sysv64" fn ap_early_entry() -> ! {
    halt_boot_cpu()
}

#[cfg(ktest)]
fn halt_boot_cpu() -> ! {
    loop {
        // SAFETY: These unsupported boot protocols must never reach a Rust entry.
        unsafe { core::arch::asm!("cli", "hlt", options(nomem, nostack)) };
    }
}
