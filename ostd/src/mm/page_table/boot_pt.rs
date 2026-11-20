// SPDX-License-Identifier: MPL-2.0
//! Because that the page table implementation requires metadata initialized
//! and mapped, the boot page table is needed to do early stage page table setup
//! in order to initialize the running phase page tables.
use vstd::prelude::*;

use super::{PageTableEntryTrait, pte_index};
use crate::arch::mm::{PageTableEntry, PagingConsts, current_page_table_paddr};
use crate::{
    mm::{
        PAGE_SIZE, Paddr, PagingConstsTrait, PagingLevel, Vaddr,
        frame::allocator,
        nr_subpage_per_huge, paddr_to_vaddr,
        page_prop::{PageFlags, PageProperty},
    },
};
use core::{
    alloc::Layout,
    result::Result,
};
use spin::Mutex;

verus! {

type FrameNumber = usize;

/// The accessor to the boot page table singleton [`BootPageTable`].
///
/// The user should provide a closure to access the boot page table. The
/// function will acquire the lock and call the closure with a mutable
/// reference to the boot page table as the argument.
///
/// The boot page table will be dropped when there's no CPU activating it.
/// This function will return an [`Err`] if the boot page table is dropped.
#[verifier::external_body]
pub(crate) fn with_borrow<F, R>(f: F) -> Result<R, ()> where
    F: FnOnce(BootPageTable) -> (R, BootPageTable),
 {
    let mut boot_pt = BOOT_PAGE_TABLE.lock();

    // Lazy initialization.

    if boot_pt.is_none() {
        // SAFETY: This function is called only once.
        *boot_pt = Some(unsafe { BootPageTable::from_current_pt() });
    }
    let (r, new_boot_pt) = f(boot_pt.take().unwrap());
    *boot_pt = Some(new_boot_pt);

    Ok(r)
}

/* Not yet supporting multiple CPUs
/// Dismiss the boot page table.
///
/// By calling it on a CPU, the caller claims that the boot page table is no
/// longer needed on this CPU.
///
/// # Safety
///
/// The caller should ensure that:
///  - another legitimate page table is activated on this CPU;
///  - this function should be called only once per CPU;
///  - no [`with_borrow`] calls are performed on this CPU after this dismissal;
///  - no [`with_borrow`] calls are performed on this CPU after the activation
///    of another page table and before this dismissal.
pub(crate) unsafe fn dismiss() {
    IS_DISMISSED.store(true);
    if DISMISS_COUNT.fetch_add(1, Ordering::SeqCst) as usize == num_cpus() - 1 {
        let boot_pt = BOOT_PAGE_TABLE.lock().take().unwrap();

        dfs_walk_on_leave::<PageTableEntry, PagingConsts>(
            boot_pt.root_pt,
            PagingConsts::NR_LEVELS,
            &mut |pte| {
                if !pte.prop().flags.contains(PTE_POINTS_TO_FIRMWARE_PT) {
                    // SAFETY: The pointed frame is allocated and forgotten with `into_raw`.
                    drop(unsafe { Frame::<EarlyAllocatedFrameMeta>::from_raw(pte.paddr()) })
                }
                // Firmware provided page tables may be a DAG instead of a tree.
                // Clear it to avoid double-free when we meet it the second time.
                *pte = PageTableEntry::new_absent();
            },
        );
    }
}
*/

/// The boot page table singleton instance.
/// A simple boot page table singleton for boot stage mapping management.
///
/// If applicable, the boot page table could track the lifetime of page table
/// frames that are set up by the firmware, loader or the setup code.
///
/// All the newly allocated page table frames have the first unused bit in
/// parent PTEs. This allows us to deallocate them when the boot page table
/// is dropped.
pub(crate) struct BootPageTable<
    E: PageTableEntryTrait = PageTableEntry,
    C: PagingConstsTrait = PagingConsts,
> {
    root_pt: FrameNumber,
    _pretend_to_use: core::marker::PhantomData<(E, C)>,
}

// We use extra two available bits in the boot PT for memory management.
//
// The first available bit is used to differentiate firmware page tables from
// the page tables allocated here. The second is for identifying double-visits
// when walking the page tables since the PT can be a DAG.
// The ktest boot page table is retained for the full test run and never dismissed.

impl<E: PageTableEntryTrait, C: PagingConstsTrait> BootPageTable<E, C> {
    /// Creates a new boot page table from the current page table root
    /// physical address.
    ///
    /// # Safety
    ///
    /// This function should be called only once in the initialization phase.
    /// Otherwise, It would lead to double-drop of the page table frames set up
    /// by the firmware, loader or the setup code.
    #[verifier::external_body]
    unsafe fn from_current_pt() -> Self {
        let root_pt = current_page_table_paddr() / C::BASE_PAGE_SIZE();
        Self { root_pt, _pretend_to_use: core::marker::PhantomData }
    }

    /// Returns the root physical address of the boot page table.
    #[verifier::external_body]
    pub(crate) fn root_address(&self) -> Paddr {
        self.root_pt * C::BASE_PAGE_SIZE()
    }

    /// Maps a base page to a frame.
    ///
    /// # Panics
    ///
    /// This function will panic if the page is already mapped.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it can cause undefined behavior if the caller
    /// maps a page in the kernel address space.
    #[verifier::external_body]
    pub unsafe fn map_base_page(&mut self, from: Vaddr, to: FrameNumber, prop: PageProperty) {
        let mut pt = self.root_pt;
        let mut level = C::NR_LEVELS();
        // Walk to the last level of the page table.
        while level > 1 {
            let index = pte_index::<C>(from, level);
            let pte_ptr = unsafe { (paddr_to_vaddr(pt * C::BASE_PAGE_SIZE()) as *mut E).add(index)
            };
            let pte = unsafe { pte_ptr.read() };
            pt =
            if !pte.is_present() {
                let pte = self.alloc_child();
                unsafe { pte_ptr.write(pte) };
                pte.paddr() / C::BASE_PAGE_SIZE()
            } else if pte.is_last(level) {
                #[cfg(feature = "allow_panic")]
                { panic!("mapping an already mapped huge page in the boot page table") }
                #[cfg(not(feature = "allow_panic"))]
                { return }
            } else {
                pte.paddr() / C::BASE_PAGE_SIZE()
            };
            level -= 1;
        }
        // Map the page in the last level page table.
        let index = pte_index::<C>(from, 1);
        let pte_ptr = unsafe { (paddr_to_vaddr(pt * C::BASE_PAGE_SIZE()) as *mut E).add(index) };
        let pte = unsafe { pte_ptr.read() };
        if pte.is_present() {
            #[cfg(feature = "allow_panic")]
            panic!("mapping an already mapped page in the boot page table");
            #[cfg(not(feature = "allow_panic"))]
             return;
        }
        unsafe { pte_ptr.write(E::new_page(to * C::BASE_PAGE_SIZE(), 1, prop)) };
    }

    /// Set protections of a base page mapping.
    ///
    /// This function may split a huge page into base pages, causing page allocations
    /// if the original mapping is a huge page.
    ///
    /// # Panics
    ///
    /// This function will panic if the page is already mapped.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it can cause undefined behavior if the caller
    /// maps a page in the kernel address space.
    #[verifier::external_body]
    pub unsafe fn protect_base_page(
        &mut self,
        virt_addr: Vaddr,
        mut op: impl FnMut(&mut PageProperty),
    ) {
        let mut pt = self.root_pt;
        let mut level = C::NR_LEVELS();
        // Walk to the last level of the page table.
        while level > 1 {
            let index = pte_index::<C>(virt_addr, level);
            let pte_ptr = unsafe { (paddr_to_vaddr(pt * C::BASE_PAGE_SIZE()) as *mut E).add(index)
            };
            let pte = unsafe { pte_ptr.read() };
            pt =
            if !pte.is_present() {
                #[cfg(feature = "allow_panic")]
                { panic!("protecting an unmapped page in the boot page table") }
                #[cfg(not(feature = "allow_panic"))]
                { return }
            } else if pte.is_last(level) {
                // Split the huge page.
                let child_pte = self.alloc_child();
                let child_frame_pa = child_pte.paddr();
                let huge_pa = pte.paddr();
                for i in 0..nr_subpage_per_huge::<C>() {
                    let nxt_ptr = unsafe { (paddr_to_vaddr(child_frame_pa) as *mut E).add(i) };
                    unsafe {
                        nxt_ptr.write(
                            E::new_page(huge_pa + i * C::BASE_PAGE_SIZE(), level - 1, pte.prop()),
                        )
                    };
                }
                unsafe { pte_ptr.write(E::new_pt(child_frame_pa)) };
                child_frame_pa / C::BASE_PAGE_SIZE()
            } else {
                pte.paddr() / C::BASE_PAGE_SIZE()
            };
            level -= 1;
        }
        // Do protection in the last level page table.
        let index = pte_index::<C>(virt_addr, 1);
        let pte_ptr = unsafe { (paddr_to_vaddr(pt * C::BASE_PAGE_SIZE()) as *mut E).add(index) };
        let pte = unsafe { pte_ptr.read() };
        if !pte.is_present() {
            #[cfg(feature = "allow_panic")]
            panic!("protecting an unmapped page in the boot page table");
            #[cfg(not(feature = "allow_panic"))]
             return;
        }
        let mut prop = pte.prop();
        op(&mut prop);
        unsafe { pte_ptr.write(E::new_page(pte.paddr(), 1, prop)) };
    }

    // TODO: come back after Allocator
    #[verifier::external_body]
    fn alloc_child(&mut self) -> E {
        let frame_paddr = allocator::early_alloc(
            Layout::from_size_align(C::BASE_PAGE_SIZE(), C::BASE_PAGE_SIZE()).unwrap(),
        )
        .unwrap();

        // Zero it out.
        let vaddr = paddr_to_vaddr(frame_paddr) as *mut u8;
        unsafe { core::ptr::write_bytes(vaddr, 0, C::BASE_PAGE_SIZE()) };

        E::new_pt(frame_paddr)

    }

    #[cfg(all(ktest, feature = "full-ktest"))]
    pub(super) fn new(root_pt: FrameNumber) -> Self {
        Self { root_pt, _pretend_to_use: core::marker::PhantomData }
    }
}

/// A helper function to walk on the page table frames.
///
/// Once leaving a page table frame, the closure will be called with the PTE to
/// the frame.
#[verifier::external_body]
fn dfs_walk_on_leave<E: PageTableEntryTrait, C: PagingConstsTrait>(
    pt: FrameNumber,
    level: PagingLevel,
    op: &mut impl FnMut(&mut E),
) {
    if level >= 2 {
        let pt_vaddr = paddr_to_vaddr(pt * C::BASE_PAGE_SIZE()) as *mut E;
        let pt = unsafe { core::slice::from_raw_parts_mut(pt_vaddr, nr_subpage_per_huge::<C>()) };
        for pte in pt {
            if pte.is_present() && !pte.is_last(level) {
                dfs_walk_on_leave::<E, C>(pte.paddr() / C::BASE_PAGE_SIZE(), level - 1, op);
                op(pte)
            }
        }
    }
}

} // verus!

static BOOT_PAGE_TABLE: Mutex<Option<BootPageTable>> = Mutex::new(None);
