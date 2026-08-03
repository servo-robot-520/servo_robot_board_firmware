//! Heap, stack watermark, and early MCU initialization.

use core::mem::MaybeUninit;
use embedded_alloc::LlffHeap as Heap;

/// Heap size in bytes.
pub const HEAP_SIZE: usize = 8192;

/// Initialize the global heap allocator with a static buffer.
///
/// # Safety
/// Must be called exactly once, before any heap allocation.
pub unsafe fn init_heap(heap: &Heap) {
    static HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        heap.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
}

/// Paint the unused stack region with `0xCC` for watermark detection.
///
/// Writes `0xCC` from the current stack pointer down to `_stack_start` (RAM end).
/// Later, `check_stack_watermark()` scans from the bottom upward to find the
/// first `0xCC` byte, which marks the high-water mark.
pub fn paint_stack() {
    unsafe extern "C" {
        static _stack_start: u32;
    }
    let sp: u32;
    unsafe { core::arch::asm!("mov {}, sp", out(reg) sp) };
    let stack_top = unsafe { &_stack_start as *const _ as u32 };
    let paint_start = sp as *mut u8;
    let paint_len = (stack_top - sp) as usize;
    unsafe {
        core::ptr::write_bytes(paint_start, 0xCC, paint_len);
    }
}

/// Scan the painted stack region and return the minimum remaining stack space
/// in **kilobytes** (rounded down).
///
/// The scan starts from the stack bottom (end of BSS/uninit) and moves upward
/// toward the stack top (RAM end), counting bytes until the first `0xCC` is found.
pub fn check_stack_watermark() -> u16 {
    unsafe extern "C" {
        static _stack_start: u32; // stack top (end of RAM)
        static _stack_end: u32; // stack bottom (end of BSS/uninit)
    }
    unsafe {
        let stack_bottom = &_stack_end as *const _ as *const u8;
        let stack_top = &_stack_start as *const _ as *const u8;
        let total = stack_top.offset_from(stack_bottom) as u32;

        // Scan from bottom upward for the first 0xCC (unused region)
        let mut scan = stack_bottom;
        while scan < stack_top && *scan != 0xCC {
            scan = scan.add(1);
        }
        let used = scan.offset_from(stack_bottom) as u32;
        ((total - used) / 1024) as u16
    }
}
