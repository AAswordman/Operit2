//! Standalone native fixture for real Dart VM ownership and ABI tests.

#[path = "../src/SharedBuffer.rs"]
mod SharedBuffer;
#[path = "../src/DartPort.rs"]
mod dart_port;
use SharedBuffer::SharedBuffer as BufferHandle;

use dart_port::{DartPort, PostCObject};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

// These probes observe only allocations explicitly selected by ownership tests.
// Fixed storage avoids allocating recursively inside the allocator's release hook.
static TRACKED_ALLOCATIONS: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
static NEXT_ALLOCATION_PROBE: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    /// Uses the system allocator without counting unrelated diagnostic allocations.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    /// Records the actual release of each watched allocation without dereferencing it.
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        for probe in &TRACKED_ALLOCATIONS {
            let _ =
                probe.compare_exchange(pointer as usize, 0, Ordering::AcqRel, Ordering::Acquire);
        }
        System.dealloc(pointer, layout);
    }

    /// Keeps a watched allocation's identity when the system allocator resizes it.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let resized = System.realloc(pointer, layout, size);
        if !resized.is_null() {
            for probe in &TRACKED_ALLOCATIONS {
                let _ = probe.compare_exchange(
                    pointer as usize,
                    resized as usize,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
            }
        }
        resized
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
    let buffers = (0..count)
        .map(|index| BufferHandle::retain(handles.add(index).read_unaligned()))
        .collect();
    DartPort::new(post, port).send(bytes, buffers)
}

/// Describes one independently attached frame in a VM delivery batch.
#[repr(C)]
pub struct FixtureFrame {
    bytes: *mut u8,
    length: usize,
    handles: *const *const BufferHandle,
    count: usize,
}

/// Sends multiple production envelopes in one native VM message.
#[no_mangle]
pub unsafe extern "C" fn fixture_send_batch(
    post: PostCObject,
    port: i64,
    frames: *const FixtureFrame,
    count: usize,
) -> bool {
    let frames = std::slice::from_raw_parts(frames, count)
        .iter()
        .map(|frame| {
            let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                frame.bytes,
                frame.length,
            ))
            .into_vec();
            let buffers = (0..frame.count)
                .map(|index| BufferHandle::retain(frame.handles.add(index).read_unaligned()))
                .collect();
            (bytes, buffers)
        })
        .collect();
    DartPort::new(post, port).sendBatch(frames)
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

/// Watches one live allocation until its real allocator release, independently of address reuse.
#[no_mangle]
pub extern "C" fn fixture_track_allocation(pointer: *mut std::ffi::c_void) -> usize {
    assert!(!pointer.is_null());
    let index = NEXT_ALLOCATION_PROBE.fetch_add(1, Ordering::Relaxed);
    TRACKED_ALLOCATIONS[index].store(pointer as usize, Ordering::Release);
    index
}

/// Reports whether the selected allocation has reached its actual native deallocation.
#[no_mangle]
pub extern "C" fn fixture_allocation_is_live(index: usize) -> bool {
    TRACKED_ALLOCATIONS[index].load(Ordering::Acquire) != 0
}
