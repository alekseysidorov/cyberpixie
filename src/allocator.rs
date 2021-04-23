//! A heap allocator for Risc-V processors, based on the `alloc-cortex-m` crate.
//!
//! Note that using this as your global allocator requires nightly Rust.

use core::{
    alloc::{GlobalAlloc, Layout},
    cell::RefCell,
    ptr::{self, NonNull},
};

use linked_list_allocator::Heap;
use riscv::interrupt::Mutex;

use crate::uprintln;

pub struct RiscVHeap {
    heap: Mutex<RefCell<Heap>>,
}

impl RiscVHeap {
    /// Crate a new UNINITIALIZED heap allocator
    ///
    /// You must initialize this heap using the
    /// [`init`](struct.RiscVHeap.html#method.init) method before using the allocator.
    pub const fn empty() -> RiscVHeap {
        RiscVHeap {
            heap: Mutex::new(RefCell::new(Heap::empty())),
        }
    }

    /// Initializes the heap
    ///
    /// This function must be called BEFORE you run any code that makes use of the
    /// allocator.
    ///
    /// `start_addr` is the address where the heap will be located.
    ///
    /// `size` is the size of the heap in bytes.
    ///
    /// Note that:
    ///
    /// - The heap grows "upwards", towards larger addresses. Thus `end_addr` must
    ///   be larger than `start_addr`
    ///
    /// - The size of the heap is `(end_addr as usize) - (start_addr as usize)`. The
    ///   allocator won't use the byte at `end_addr`.
    ///
    /// # Safety
    ///
    /// Obey these or Bad Stuff will happen.
    ///
    /// - This function must be called exactly ONCE.
    /// - `size > 0`
    pub unsafe fn init(&self, start_addr: usize, size: usize) {
        riscv::interrupt::free(|cs| {
            self.heap.borrow(cs).borrow_mut().init(start_addr, size);
        });
    }

    /// Returns an estimate of the amount of bytes in use.
    pub fn used(&self) -> usize {
        riscv::interrupt::free(|cs| self.heap.borrow(cs).borrow_mut().used())
    }

    /// Returns an estimate of the amount of bytes available.
    pub fn free(&self) -> usize {
        riscv::interrupt::free(|cs| self.heap.borrow(cs).borrow_mut().free())
    }
}

unsafe impl GlobalAlloc for RiscVHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        riscv::interrupt::free(|cs| {
            uprintln!("alloc: {:?}, used: {}, free: {}", layout, self.used(), self.free());
            self.heap
                .borrow(cs)
                .borrow_mut()
                .allocate_first_fit(layout)
                .ok()
                .map_or(ptr::null_mut(), |allocation| allocation.as_ptr())
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        riscv::interrupt::free(|cs| {
            uprintln!("dealloc: {:?}, used: {}, free: {}", layout, self.used(), self.free());
            self.heap
                .borrow(cs)
                .borrow_mut()
                .deallocate(NonNull::new_unchecked(ptr), layout)
        });
    }
}

#[inline]
pub fn heap_bottom() -> usize {
    extern "C" {
        static _sheap: u8;
    }
   
    unsafe { &_sheap as *const u8 as usize }
}
