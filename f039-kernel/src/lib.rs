#![no_std]

use core::{
    alloc::Layout,
    sync::atomic::{AtomicUsize, Ordering},
};
use ostd::{
    global_frame_allocator,
    mm::{FrameAllocOptions, PAGE_SIZE, Paddr, frame::{GlobalFrameAllocator, Link, LinkedList, UniqueFrame}},
    specs::mm::frame::linked_list::linked_list_owners::MetaSlotSmall,
};

struct Region {
    cursor: AtomicUsize,
    end: AtomicUsize,
}

impl Region {
    const fn new() -> Self {
        Self { cursor: AtomicUsize::new(0), end: AtomicUsize::new(0) }
    }
}

struct TestFrameAllocator {
    count: AtomicUsize,
    regions: [Region; 128],
}

impl TestFrameAllocator {
    const fn new() -> Self {
        Self { count: AtomicUsize::new(0), regions: [const { Region::new() }; 128] }
    }
}

impl GlobalFrameAllocator for TestFrameAllocator {
    fn alloc(&self, layout: Layout) -> Option<Paddr> {
        for region in &self.regions[..self.count.load(Ordering::Acquire)] {
            let mut cursor = region.cursor.load(Ordering::Relaxed);
            loop {
                let aligned = cursor.checked_add(layout.align().saturating_sub(1))?
                    & !layout.align().saturating_sub(1);
                let end = aligned.checked_add(layout.size())?;
                if end > region.end.load(Ordering::Acquire) {
                    break;
                }
                match region.cursor.compare_exchange_weak(
                    cursor, end, Ordering::AcqRel, Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(aligned),
                    Err(actual) => cursor = actual,
                }
            }
        }
        None
    }

    fn dealloc(&self, _addr: Paddr, _size: usize) {}

    fn add_free_memory(&self, addr: Paddr, size: usize) {
        let index = self.count.load(Ordering::Relaxed);
        assert!(index < self.regions.len(), "too many free memory ranges");
        let region = &self.regions[index];
        region.cursor.store(addr, Ordering::Relaxed);
        region.end.store(addr.checked_add(size).expect("memory range overflow"), Ordering::Relaxed);
        self.count.store(index + 1, Ordering::Release);
    }
}

#[global_frame_allocator]
static FRAME_ALLOCATOR: TestFrameAllocator = TestFrameAllocator::new();

#[ostd::main]
fn kernel_main() {
    linked_list_contains_membership();
    ostd::early_println!("F039_ORIGINAL_PASS");
    qemu_exit(0x10)
}

fn linked_list_contains_membership() {
    let alloc_options = FrameAllocOptions::new();
    let frame1 = alloc_options
        .alloc_frame_with(Link::new(MetaSlotSmall))
        .unwrap();
    let frame1_addr = frame1.start_paddr();
    let frame2 = alloc_options
        .alloc_frame_with(Link::new(MetaSlotSmall))
        .unwrap();
    let frame2_addr = frame2.start_paddr();
    let frame3 = alloc_options
        .alloc_frame_with(Link::new(MetaSlotSmall))
        .unwrap();
    let frame3_addr = frame3.start_paddr();
    let frame_outside = alloc_options
        .alloc_frame_with(Link::new(MetaSlotSmall))
        .unwrap();
    let frame_outside_addr = frame_outside.start_paddr();

    let mut list = LinkedList::new();
    let unique1: UniqueFrame<Link<MetaSlotSmall>> = match frame1.try_into() {
        Ok(frame) => frame,
        Err(_) => panic!("failed to convert first frame"),
    };
    let unique2: UniqueFrame<Link<MetaSlotSmall>> = match frame2.try_into() {
        Ok(frame) => frame,
        Err(_) => panic!("failed to convert second frame"),
    };
    let unique3: UniqueFrame<Link<MetaSlotSmall>> = match frame3.try_into() {
        Ok(frame) => frame,
        Err(_) => panic!("failed to convert third frame"),
    };
    list.push_front(unique1);
    list.push_front(unique2);
    list.push_front(unique3);

    assert!(list.contains(frame1_addr));
    assert!(list.contains(frame2_addr));
    assert!(list.contains(frame3_addr));
    assert!(!list.contains(frame_outside_addr));
    // An aligned physical address beyond the metadata table exercises the
    // missing-slot early return (F-039 G-001).
    assert!(!list.contains(usize::MAX & !(PAGE_SIZE - 1)));
}

#[ostd::panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    ostd::early_println!("F039_FAIL: {}", info);
    qemu_exit(0x11)
}

fn qemu_exit(value: u32) -> ! {
    // SAFETY: QEMU's isa-debug-exit device is configured at port 0xf4.
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") value);
    }
    loop {
        // SAFETY: This is the terminal path after signaling the host.
        unsafe { core::arch::asm!("cli", "hlt", options(nomem, nostack)) };
    }
}
