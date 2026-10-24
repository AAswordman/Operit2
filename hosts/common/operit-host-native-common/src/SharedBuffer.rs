//! Shared binary ownership for the native Rust–Dart ABI.

use bytes::Bytes;
use std::ffi::c_void;

pub struct SharedBuffer {
    bytes: Bytes,
    sealed: bool,
    receipt: u8,
}

impl SharedBuffer {
    /// Retains immutable bytes without copying their allocation.
    pub fn sealed(bytes: Bytes) -> Box<Self> {
        Box::new(Self {
            bytes,
            sealed: true,
            receipt: 0,
        })
    }

    /// Retains a submitted immutable allocation before its Dart owner can disappear.
    pub unsafe fn retain(pointer: *const Self) -> Bytes {
        assert!(!pointer.is_null(), "shared buffer handle must be live");
        assert!(
            (*pointer).sealed,
            "shared buffer must be sealed before submission"
        );
        (*pointer).bytes.clone()
    }

    /// Returns the immutable native storage published to the VM.
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    /// Exposes an initialized ownership receipt without charging the payload again.
    pub fn receipt(&self) -> *const u8 {
        &self.receipt
    }
}

/// Allocates one mutable ingress buffer; it must be sealed after Dart fills it.
pub unsafe extern "C" fn shared_allocate(length: usize) -> *mut SharedBuffer {
    Box::into_raw(Box::new(SharedBuffer {
        bytes: vec![0; length].into(),
        sealed: false,
        receipt: 0,
    }))
}

/// Creates independent immutable ownership for a newly adopted Dart view.
pub unsafe extern "C" fn shared_retain(pointer: *const SharedBuffer) -> *mut SharedBuffer {
    Box::into_raw(SharedBuffer::sealed(SharedBuffer::retain(pointer)))
}

/// Exposes storage only while its single Dart producer is filling it.
pub unsafe extern "C" fn shared_data(pointer: *mut SharedBuffer) -> *mut u8 {
    assert!(
        !pointer.is_null() && !(*pointer).sealed,
        "buffer is not writable"
    );
    (*pointer).bytes.as_ptr().cast_mut()
}

/// Publishes the fully initialized ingress buffer as immutable shared storage.
pub unsafe extern "C" fn shared_seal(pointer: *mut SharedBuffer) {
    assert!(
        !pointer.is_null() && !(*pointer).sealed,
        "buffer is already sealed"
    );
    (*pointer).sealed = true;
}

/// Releases one handle after its external typed-data view becomes unreachable.
pub unsafe extern "C" fn shared_release(pointer: *mut c_void) {
    drop(Box::from_raw(pointer.cast::<SharedBuffer>()));
}

/// Adapts the native-port finalizer to the same shared-buffer release operation.
pub unsafe extern "C" fn shared_port_release(_: *mut c_void, pointer: *mut c_void) {
    shared_release(pointer);
}
