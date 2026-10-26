//! Minimal ABI for Dart's thread-safe NativeApi.postCObject entry point.
//! Layouts follow dart_native_api.h; immutable payload ownership passes to the VM.

use crate::SharedBuffer::{shared_port_release, SharedBuffer};
use bytes::Bytes;
use std::ffi::c_void;

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

    /// Sends one frame through the uniform ABI-v3 batch envelope.
    pub fn send(self, bytes: Vec<u8>, buffers: Vec<Bytes>) -> bool {
        self.sendBatch(vec![(bytes, buffers)])
    }

    /// Transfers ordered frame envelopes together without copying their binary allocations.
    #[allow(non_snake_case)]
    pub fn sendBatch(self, frames: Vec<(Vec<u8>, Vec<Bytes>)>) -> bool {
        let mut peers = Vec::new();
        let mut frames: Vec<Vec<DartObject>> = frames
            .into_iter()
            .map(|(bytes, buffers)| frame_objects(bytes, buffers, &mut peers))
            .collect();
        let mut pointers: Vec<Vec<*mut DartObject>> = frames
            .iter_mut()
            .map(|frame| frame.iter_mut().map(|object| object as *mut _).collect())
            .collect();
        let mut envelopes: Vec<DartObject> = pointers
            .iter_mut()
            .map(|frame| array_object(frame))
            .collect();
        let mut batch: Vec<*mut DartObject> = envelopes
            .iter_mut()
            .map(|object| object as *mut _)
            .collect();
        let mut message = array_object(&mut batch);
        let accepted = unsafe { (self.post)(self.id, &mut message) };
        if !accepted {
            for peer in peers {
                unsafe { shared_port_release(std::ptr::null_mut(), peer) };
            }
        }
        accepted
    }
}

/// Builds one frame's metadata and independently scoped shared-buffer receipts.
fn frame_objects(
    bytes: Vec<u8>,
    buffers: Vec<Bytes>,
    peers: &mut Vec<*mut c_void>,
) -> Vec<DartObject> {
    let mut objects = Vec::with_capacity(1 + 4 * buffers.len());
    objects.push(external_object(bytes.into(), peers));
    for bytes in buffers {
        let payload = SharedBuffer::sealed(bytes);
        let address = payload.bytes().as_ptr() as i64;
        let length = payload.bytes().len() as i64;
        let data = payload.receipt().cast_mut();
        let peer = Box::into_raw(payload).cast::<c_void>();
        peers.push(peer);
        objects.push(integer_object(peer as i64));
        objects.push(DartObject {
            kind: 13,
            value: ObjectValue {
                external_typed_data: ExternalTypedData {
                    kind: 2,
                    length: 1,
                    data,
                    peer,
                    callback: shared_port_release,
                },
            },
        });
        objects.push(integer_object(address));
        objects.push(integer_object(length));
    }
    objects
}

/// Encodes a borrowed array whose backing pointers remain alive through the VM post call.
fn array_object(values: &mut Vec<*mut DartObject>) -> DartObject {
    DartObject {
        kind: 6,
        value: ObjectValue {
            array: ObjectArray {
                length: values.len() as isize,
                values: values.as_mut_ptr(),
            },
        },
    }
}

/// Encodes one native address or length without touching binary content.
fn integer_object(integer: i64) -> DartObject {
    DartObject {
        kind: 3,
        value: ObjectValue { integer },
    }
}

/// Creates a VM-owned view while retaining only a small native ownership handle.
fn external_object(bytes: Bytes, peers: &mut Vec<*mut c_void>) -> DartObject {
    let payload = SharedBuffer::sealed(bytes);
    let data = payload.bytes().as_ptr().cast_mut();
    let length = payload.bytes().len() as isize;
    let peer = Box::into_raw(payload).cast::<c_void>();
    peers.push(peer);
    DartObject {
        kind: 13,
        value: ObjectValue {
            external_typed_data: ExternalTypedData {
                kind: 2,
                length,
                data,
                peer,
                callback: shared_port_release,
            },
        },
    }
}
