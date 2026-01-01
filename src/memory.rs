use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

pub static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static A: TrackingAllocator = TrackingAllocator;

#[derive(Debug, Clone, Copy)]
pub struct Gc<T> {
    ptr: NonNull<T>,
}

impl<T> Gc<T> {
    pub fn new(value: T) -> Self {
        let boxed = Box::new(value);
        Gc {
            ptr: NonNull::new(Box::into_raw(boxed)).expect("Box::into_raw returned null"),
        }
    }

    pub fn as_ptr(self) -> *const T {
        unsafe { self.ptr.as_ref() as *const T }
    }

    pub fn ptr_eq(&self, other: &Gc<T>) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl<T> std::ops::Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> std::ops::DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}
