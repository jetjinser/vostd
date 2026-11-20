// SPDX-License-Identifier: MPL-2.0
//! The standard library for Asterinas and other Rust OSes.
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(btree_cursors)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(coroutines)]
#![feature(fn_traits)]
#![feature(iter_advance_by)]
#![feature(iter_from_coroutine)]
#![feature(linkage)]
#![feature(min_specialization)]
//#![feature(negative_impls)]
#![feature(ptr_metadata)]
#![feature(sync_unsafe_cell)]
#![expect(internal_features)]
#![no_std]
// Disable `missing_docs`: verification-only `inv`/`type_inv` items are
// explained by proof context rather than API docs.
// #![warn(missing_docs)]
#![feature(proc_macro_hygiene)]
#![allow(unused_parens)]
#![allow(unused_braces)]
#![allow(rustdoc::invalid_rust_codeblocks)]
#![allow(rustdoc::invalid_html_tags)]
#![allow(rustdoc::broken_intra_doc_links)]
// Allow Verus-style specification names that mirror constants or proof-level
// concepts rather than Rust's conventional function naming.
#![allow(non_snake_case)]

extern crate alloc;

//#[cfg(target_arch = "x86_64")]
#[path = "arch/x86/mod.rs"]
pub mod arch;
//#[cfg(target_arch = "riscv64")]
//#[path = "arch/riscv/mod.rs"]
//pub mod arch;
pub mod boot;
#[path = "console.rs"]
pub mod console;
pub mod cpu;
pub mod error;
pub mod mm;
#[cfg(not(ktest))]
pub mod panic;
#[cfg(ktest)]
#[path = "panic_ktest.rs"]
pub mod panic;
#[cfg(not(ktest))]
pub mod power;
pub mod prelude;
#[cfg(not(ktest))]
pub mod smp;
pub mod sync;
pub mod task;
pub mod timer;
pub mod trap;
#[path = "../specs/mod.rs"]
pub mod specs;
pub mod util;

use core::sync::atomic::{AtomicBool, Ordering};

pub use ostd_macros::{global_frame_allocator, main, panic_handler};
pub use ostd_pod::Pod;
pub use self::error::Error;

/// Initializes OSTD for the minimal ktest branch.
#[doc(hidden)]
unsafe fn init() {
    #[cfg(not(ktest))]
    arch::enable_cpu_features();
    #[cfg(all(ktest, target_arch = "x86_64"))]
    // SAFETY: The x86-64 ktest path requires NXE for non-executable page mappings.
    unsafe { arch::enable_nxe_for_ktest() };

    // SAFETY: This function is called only once, before `allocator::init`
    // and after memory regions are initialized.
    unsafe { mm::frame::allocator::init_early_allocator() };

    #[cfg(target_arch = "x86_64")]
    arch::serial::init();
    #[cfg(not(target_arch = "x86_64"))]
    arch::serial::init();

    // SAFETY:
    // 1. They are only called once in the boot context of the BSP.
    // 2. The number of CPUs are available because ACPI has been initialized.
    // 3. No CPU-local objects have been accessed yet.
    #[cfg(not(ktest))]
    unsafe { cpu::init_on_bsp() };

    // SAFETY: We are on the BSP and APs are not yet started.
    let meta_pages = unsafe { mm::frame::meta::init() };
    unsafe { mm::frame::allocator::init() };

    mm::kspace::init_kernel_page_table(meta_pages);

    sync::init();

    #[cfg(not(ktest))]
    boot::init_after_heap();

    #[cfg(not(ktest))]
    mm::dma::init();

    #[cfg(not(ktest))]
    unsafe { arch::late_init_on_bsp() };

    #[cfg(not(ktest))]
    smp::init();

    // SAFETY: This function is called only once on the BSP.
    #[cfg(not(ktest))]
    unsafe {
        mm::kspace::activate_kernel_page_table();
    }

    #[cfg(not(ktest))]
    arch::irq::enable_local();

    IN_BOOTSTRAP_CONTEXT.store(false, Ordering::Relaxed);
}

pub(crate) static IN_BOOTSTRAP_CONTEXT: AtomicBool = AtomicBool::new(true);

#[doc(hidden)]
pub mod ktest {
    pub use ostd_macros::{test_main as main, test_panic_handler as panic_handler};
    pub use ostd_test::*;
}
