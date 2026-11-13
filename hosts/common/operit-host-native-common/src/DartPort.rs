//! Minimal ABI for Dart's thread-safe NativeApi.postCObject entry point.
//! Layouts follow dart_native_api.h; immutable payload ownership passes to the VM.

use crate::SharedBuffer::{shared_port_release, SharedBuffer};
use bytes::Bytes;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

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
        // Save only protocol headers before moving payloads into VM-owned buffers.
        let headers: Vec<_> = frames
            .iter()
            .take(128)
            .map(|(bytes, _)| {
                (
                    bytes.first().copied(),
                    bytes.get(1..9).map(|id| {
                        i64::from_le_bytes(id.try_into().expect("nine-byte frame header"))
                    }),
                )
            })
            .collect();
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
        let diagnostic =
            PostDiagnostic::capture(self.id, &message, &envelopes, &frames, &pointers, &headers);
        let sequence = diagnostic.sequence;
        {
            let mut history = post_history()
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if history.len() == 32 {
                history.pop_front();
            }
            history.push_back(diagnostic);
        }
        let accepted = unsafe { (self.post)(self.id, &mut message) };
        // The VM may already have finalized peers. Never inspect their data after posting.
        if let Some(diagnostic) = post_history()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter_mut()
            .find(|entry| entry.sequence == sequence)
        {
            diagnostic.accepted = Some(accepted);
        }
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

// The flight recorder owns scalar snapshots only, never pointers or payload allocations.
// It is process-wide and bounded to 32 batches, 128 frames and one receipt per frame.
static POST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn post_history() -> &'static Mutex<VecDeque<PostDiagnostic>> {
    static HISTORY: OnceLock<Mutex<VecDeque<PostDiagnostic>>> = OnceLock::new();
    HISTORY.get_or_init(|| Mutex::new(VecDeque::with_capacity(32)))
}

#[derive(Clone)]
struct ObjectDiagnostic {
    address: usize,
    kind: i32,
    typed_kind: Option<i32>,
    length: Option<isize>,
    data: Option<usize>,
    peer: Option<usize>,
}

impl ObjectDiagnostic {
    fn capture(object: &DartObject) -> Self {
        let mut result = Self {
            address: object as *const _ as usize,
            kind: object.kind,
            typed_kind: None,
            length: None,
            data: None,
            peer: None,
        };
        if object.kind == 13 {
            // Read the initialized descriptor union, never the external allocation itself.
            let external = unsafe { object.value.external_typed_data };
            result.typed_kind = Some(external.kind);
            result.length = Some(external.length);
            result.data = Some(external.data as usize);
            result.peer = Some(external.peer as usize);
        }
        result
    }

    fn describe(&self) -> String {
        format!(
            "object=0x{:x} type={} typedType={:?} length={:?} data={:x?} peer={:x?}",
            self.address, self.kind, self.typed_kind, self.length, self.data, self.peer
        )
    }
}

#[derive(Clone)]
struct FrameDiagnostic {
    envelope: usize,
    values: usize,
    slots: usize,
    header: (Option<u8>, Option<i64>),
    metadata: Option<ObjectDiagnostic>,
    first_receipt: Option<ObjectDiagnostic>,
    first_bad_slot: Option<usize>,
    pointer_mismatches: usize,
}

#[derive(Clone)]
struct PostDiagnostic {
    sequence: u64,
    port: i64,
    unix_millis: u128,
    root: usize,
    values: usize,
    frame_count: usize,
    frames: Vec<FrameDiagnostic>,
    accepted: Option<bool>,
}

impl PostDiagnostic {
    fn capture(
        port: i64,
        root: &DartObject,
        envelopes: &[DartObject],
        frames: &[Vec<DartObject>],
        pointers: &[Vec<*mut DartObject>],
        headers: &[(Option<u8>, Option<i64>)],
    ) -> Self {
        // These arrays were constructed immediately above and have not been passed to the VM.
        let root_array = unsafe { root.value.array };
        let snapshots = frames
            .iter()
            .zip(envelopes)
            .zip(pointers)
            .zip(headers)
            .map(|(((frame, envelope), values), header)| {
                let array = unsafe { envelope.value.array };
                FrameDiagnostic {
                    envelope: envelope as *const _ as usize,
                    values: array.values as usize,
                    slots: frame.len(),
                    header: *header,
                    metadata: frame.first().map(ObjectDiagnostic::capture),
                    first_receipt: frame.get(2).map(ObjectDiagnostic::capture),
                    first_bad_slot: frame.iter().enumerate().position(|(slot, object)| {
                        let expected = if slot == 0 || slot % 4 == 2 { 13 } else { 3 };
                        object.kind != expected
                            || (expected == 13
                                && unsafe { object.value.external_typed_data.kind != 2 })
                    }),
                    pointer_mismatches: frame
                        .iter()
                        .zip(values)
                        .filter(|(object, pointer)| {
                            **pointer as usize != *object as *const DartObject as usize
                        })
                        .count(),
                }
            })
            .collect();
        Self {
            sequence: POST_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            port,
            unix_millis: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            root: root as *const _ as usize,
            values: root_array.values as usize,
            frame_count: frames.len(),
            frames: snapshots,
            accepted: None,
        }
    }
}

/// Copies the bounded flight recorder before the error logger can trigger more deliveries.
/// Formatting and logging happen outside its lock and never dereference captured addresses.
pub fn recent_post_diagnostics(port: Option<i64>) -> Vec<String> {
    let history: Vec<_> = post_history()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .filter(|entry| port.is_none_or(|port| entry.port == port))
        .cloned()
        .collect();
    let mut lines = vec![format!(
        "ffi_tx_history diagnostics=ffi-v1 port={port:?} retained={} capacity=32 frameLimit=128 arch={} pointerBytes={} objectBytes={} arrayType=6 externalType=13 uint8Type=2",
        history.len(), std::env::consts::ARCH, std::mem::size_of::<usize>(), std::mem::size_of::<DartObject>()
    )];
    for entry in history {
        lines.push(format!(
            "ffi_tx_batch post={} nativePort={} unixMs={} accepted={:?} root=0x{:x} values=0x{:x} frames={} captured={}",
            entry.sequence, entry.port, entry.unix_millis, entry.accepted, entry.root, entry.values,
            entry.frame_count, entry.frames.len()
        ));
        for (index, frame) in entry.frames.iter().enumerate() {
            lines.push(format!(
                "ffi_tx_frame post={} index={index} envelope=0x{:x} values=0x{:x} slots={} kind={:?} id={:?} firstBadSlot={:?} pointerMismatches={} metadata=[{}] receipt=[{}]",
                entry.sequence, frame.envelope, frame.values, frame.slots, frame.header.0, frame.header.1,
                frame.first_bad_slot, frame.pointer_mismatches,
                frame.metadata.as_ref().map(ObjectDiagnostic::describe).unwrap_or_default(),
                frame.first_receipt.as_ref().map(ObjectDiagnostic::describe).unwrap_or_default(),
            ));
        }
    }
    lines
}
