use std::alloc::{GlobalAlloc, Layout, System};
use std::ops::DerefMut;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::table::{GlobalVariableTable, StringTable};
use crate::value::{Obj, ObjType, Value, ValueArray};

struct TrackingAllocator;

pub const GC_HEAP_GROW_FACTOR: usize = 2;
pub static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
pub static NEXT_GC: AtomicUsize = AtomicUsize::new(1024 * 1024);
pub static GC_REQUESTED: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "debug_stress_gc")]
        GC_REQUESTED.store(true, Ordering::Relaxed);

        if ALLOCATED.load(Ordering::Relaxed) > NEXT_GC.load(Ordering::Relaxed) {
            GC_REQUESTED.store(true, Ordering::Relaxed);
        }

        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
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

// --- GC marking functions ---
pub fn mark_value(value: &mut Value, gc_gray_stack: &mut Vec<Gc<Obj>>) {
    if let Value::Obj(obj) = value
        && !obj.is_marked
    {
        mark_object(obj.clone(), gc_gray_stack);
    }
}

pub fn mark_object(mut obj: Gc<Obj>, gc_gray_stack: &mut Vec<Gc<Obj>>) {
    if obj.is_marked {
        return;
    }

    #[cfg(feature = "debug_log_gc")]
    {
        use crate::debug::print_value;

        print!("{:?} mark ", obj);
        print_value(&Value::new_obj(obj.clone()));
        println!();
    }

    obj.is_marked = true;
    gc_gray_stack.push(obj);
}

pub fn mark_value_array(value_array: &mut ValueArray, gc_gray_stack: &mut Vec<Gc<Obj>>) {
    for value in &mut value_array.values {
        mark_value(value, gc_gray_stack);
    }
}

pub fn mark_global_table(global_table: &mut GlobalVariableTable, gc_gray_stack: &mut Vec<Gc<Obj>>) {
    for value in global_table.table.values_mut() {
        mark_value(value, gc_gray_stack);
    }
}

pub fn trace_reference(gc_gray_stack: &mut Vec<Gc<Obj>>) {
    while let Some(obj) = gc_gray_stack.pop() {
        blacken_object(obj, gc_gray_stack);
    }
}

pub fn blacken_object(mut obj: Gc<Obj>, gc_gray_stack: &mut Vec<Gc<Obj>>) {
    #[cfg(feature = "debug_log_gc")]
    {
        use crate::debug::print_value;

        print!("{:?} blacken ", obj);
        print_value(&Value::new_obj(obj.clone()));
        println!();
    }

    match &mut obj.deref_mut().obj_type {
        ObjType::Function(function) => {
            if let Some(name) = &function.name {
                mark_object(name.clone(), gc_gray_stack);
            }
            mark_value_array(&mut function.chunk.constants, gc_gray_stack);
        }
        ObjType::Closure(closure) => {
            mark_object(closure.function.clone(), gc_gray_stack);
            for upvalue in &closure.upvalues {
                mark_object(upvalue.clone(), gc_gray_stack);
            }
        }
        ObjType::Upvalue(upvalue) => {
            if let Some(closed) = &mut upvalue.closed {
                mark_value(closed, gc_gray_stack);
            }
        }
        ObjType::String(_) | ObjType::Native(_) | ObjType::Class(_) => {}
    }
}

pub fn remove_white_strings(strings: &mut StringTable) {
    strings.table.retain(|_, obj| obj.is_marked);
}

pub fn sweep(head: &mut Option<Gc<Obj>>) {
    let mut previous: Option<Gc<Obj>> = None;
    let mut current: Option<Gc<Obj>> = head.clone();

    while let Some(mut obj) = current {
        if obj.is_marked {
            obj.is_marked = false;
            previous = Some(obj.clone());
            current = obj.next.clone();
        } else {
            #[cfg(feature = "debug_log_gc")]
            {
                use crate::debug::print_value;

                print!("{:?} free ", obj);
                print_value(&Value::new_obj(obj.clone()));
                println!();
            }

            let next = obj.next.clone();
            if let Some(prev) = previous.as_mut() {
                prev.next = next.clone();
            } else {
                *head = next.clone();
            }

            free_obj(obj);

            current = next;
        }
    }
}

pub fn free_obj(obj: Gc<Obj>) {
    let _boxed: Box<Obj> = unsafe { Box::from_raw(obj.ptr.as_ptr()) };
}
