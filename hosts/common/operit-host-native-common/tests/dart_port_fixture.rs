//! Standalone native fixture for real Dart VM ownership and ABI tests.

#[path = "../src/DartPort.rs"]
mod dart_port;
#[path = "../src/SharedBuffer.rs"]
mod SharedBuffer;
use SharedBuffer::SharedBuffer as BufferHandle;

use dart_port::{DartPort, PostCObject};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static LIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    /// Counts successful allocations owned by this isolated fixture library.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            LIVE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    /// Counts releases made by the production payload finalizer.
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
        LIVE_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
    }

    /// Resizes one existing allocation without changing the live count.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        System.realloc(pointer, layout, size)
    }
}

/// Allocates an owned byte buffer to be consumed by the production sender.
#[no_mangle]
pub unsafe extern "C" fn fixture_allocate(length: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; length].into_boxed_slice()).cast::<u8>()
}

/// Sends the original allocation with the production ABI and native finalizer.
#[no_mangle]
pub unsafe extern "C" fn fixture_send(
    post: PostCObject,
    port: i64,
    pointer: *mut u8,
    length: usize,
    handles: *const *const BufferHandle,
    count: usize,
) -> bool {
    let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer, length)).into_vec();
    let buffers = (0..count).map(|index| BufferHandle::retain(handles.add(index).read_unaligned())).collect();
    DartPort::new(post, port).send(bytes, buffers)
}

/// Allocates native ingress ownership using the production buffer implementation.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_allocate(length: usize) -> *mut BufferHandle {
    SharedBuffer::shared_allocate(length)
}

/// Provides the sole producer's writable view before sealing.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_data(handle: *mut BufferHandle) -> *mut u8 {
    SharedBuffer::shared_data(handle)
}

/// Publishes the initialized data using the production immutable transition.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_seal(handle: *mut BufferHandle) {
    SharedBuffer::shared_seal(handle)
}

/// Releases one test or VM-owned reference through the production finalizer.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_release(handle: *mut std::ffi::c_void) {
    SharedBuffer::shared_release(handle)
}

/// Retains an incoming handle while the fixture dispatches its Dart test callback.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_retain(handle: *const BufferHandle) -> *mut BufferHandle {
    Box::into_raw(BufferHandle::sealed(BufferHandle::retain(handle)))
}

/// Reports the backing address for zero-copy ownership checks.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_view(handle: *const BufferHandle) -> *const u8 {
    (*handle).bytes().as_ptr()
}

/// Reports the allocation length for constructing a test host's retained view.
#[no_mangle]
pub unsafe extern "C" fn fixture_buffer_length(handle: *const BufferHandle) -> usize {
    (*handle).bytes().len()
}

/// Reports native ownership independently of Dart references or connection state.
#[no_mangle]
pub extern "C" fn fixture_live_allocations() -> usize {
    LIVE_ALLOCATIONS.load(Ordering::Relaxed)
}
