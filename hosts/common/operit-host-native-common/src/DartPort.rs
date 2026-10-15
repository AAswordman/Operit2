//! Minimal ABI for Dart's thread-safe NativeApi.postCObject entry point.
//! Layouts follow dart_native_api.h; immutable payload ownership passes to the VM.

use std::ffi::c_void;
use bytes::Bytes;
use crate::SharedBuffer::{SharedBuffer, shared_port_release};

#[repr(C)]
#[derive(Clone, Copy)]
struct ObjectArray {
    length: isize,
    values: *mut *mut DartObject,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ExternalTypedData {
    kind: i32,
    length: isize,
    data: *mut u8,
    peer: *mut c_void,
    callback: unsafe extern "C" fn(*mut c_void, *mut c_void),
}

#[repr(C)]
union ObjectValue {
    external_typed_data: ExternalTypedData,
    integer: i64,
    double: f64,
    array: ObjectArray,
}

#[repr(C)]
pub struct DartObject {
    kind: i32,
    value: ObjectValue,
}

pub type PostCObject = unsafe extern "C" fn(i64, *mut DartObject) -> bool;

#[derive(Clone, Copy)]
pub struct DartPort {
    post: PostCObject,
    id: i64,
}

impl DartPort {
    /// Binds a VM-provided post function to a Dart SendPort's native identifier.
    pub fn new(post: PostCObject, id: i64) -> Self {
        Self { post, id }
    }

    /// Transfers an immutable byte frame without copying its allocation into the VM.
    pub fn send(self, bytes: Vec<u8>, buffers: Vec<Bytes>) -> bool {
        let mut peers = Vec::with_capacity(1 + buffers.len());
        let mut objects = Vec::with_capacity(1 + 4 * buffers.len());
        objects.push(external_object(bytes.into(), &mut peers));
        for bytes in buffers {
            let payload = SharedBuffer::sealed(bytes);
            let address = payload.bytes().as_ptr() as i64;
            let length = payload.bytes().len() as i64;
            let data = payload.receipt().cast_mut();
            let peer = Box::into_raw(payload).cast::<c_void>();
            peers.push(peer);
            objects.push(integer_object(peer as i64));
            objects.push(DartObject { kind: 13, value: ObjectValue { external_typed_data: ExternalTypedData {
                kind: 2, length: 1, data, peer, callback: shared_port_release,
            } } });
            objects.push(integer_object(address));
            objects.push(integer_object(length));
        }
        let mut pointers: Vec<*mut DartObject> = objects.iter_mut().map(|item| item as *mut _).collect();
        let mut message = DartObject { kind: 6, value: ObjectValue { array: ObjectArray {
            length: pointers.len() as isize, values: pointers.as_mut_ptr(),
        } } };
        let accepted = unsafe { (self.post)(self.id, &mut message) };
        if !accepted {
            for peer in peers { unsafe { shared_port_release(std::ptr::null_mut(), peer) }; }
        }
        accepted
    }
}

/// Encodes one native address or length without touching binary content.
fn integer_object(integer: i64) -> DartObject {
    DartObject { kind: 3, value: ObjectValue { integer } }
}

/// Creates a VM-owned view while retaining only a small native ownership handle.
fn external_object(bytes: Bytes, peers: &mut Vec<*mut c_void>) -> DartObject {
    let payload = SharedBuffer::sealed(bytes);
    let data = payload.bytes().as_ptr().cast_mut();
    let length = payload.bytes().len() as isize;
    let peer = Box::into_raw(payload).cast::<c_void>();
    peers.push(peer);
    DartObject { kind: 13, value: ObjectValue { external_typed_data: ExternalTypedData {
        kind: 2, length, data, peer, callback: shared_port_release,
    } } }
}
