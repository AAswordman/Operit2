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
        // The VM's ApiMessageSerializer keys its WeakTable with ObjectPtr. With compressed
        // pointers, aligned native addresses compare by their low 32 bits. Keep EVERY
        // descriptor, including the root, in one allocation smaller than 4 GiB so distinct
        // C objects cannot alias when the VM assigns references. Payloads stay zero-copy.
        let frame_count = frames.len();
        let object_count = frames
            .iter()
            .try_fold(1 + frame_count, |count, (_, buffers)| {
                count
                    .checked_add(1)?
                    .checked_add(buffers.len().checked_mul(4)?)
            })
            .expect("Dart descriptor count must fit in address space");
        assert!(
            object_count <= u32::MAX as usize / std::mem::size_of::<DartObject>(),
            "Dart descriptor graph must fit within a 32-bit address span"
        );
        let mut objects = Vec::with_capacity(object_count);
        objects.resize_with(1 + frame_count, || integer_object(0));
        let mut peers = Vec::new();
        let mut ranges = Vec::with_capacity(frame_count);
        for (bytes, buffers) in frames {
            let start = objects.len();
            frame_objects(bytes, buffers, &mut peers, &mut objects);
            ranges.push(start..objects.len());
        }
        let base = objects.as_mut_ptr();
        let mut pointers: Vec<Vec<*mut DartObject>> = ranges
            .iter()
            .map(|range| {
                range
                    .clone()
                    .map(|index| unsafe { base.add(index) })
                    .collect()
            })
            .collect();
        for (index, frame) in pointers.iter_mut().enumerate() {
            unsafe { base.add(1 + index).write(array_object(frame)) };
        }
        let mut batch: Vec<*mut DartObject> = (1..1 + frame_count)
            .map(|index| unsafe { base.add(index) })
            .collect();
        unsafe { base.write(array_object(&mut batch)) };
        let frames: Vec<_> = ranges.iter().map(|range| &objects[range.clone()]).collect();
        let diagnostic = PostDiagnostic::capture(
            self.id,
            &objects[0],
            &objects[1..1 + frame_count],
            &frames,
            &pointers,
            &headers,
        );
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
        let accepted = unsafe { (self.post)(self.id, base) };
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
    objects: &mut Vec<DartObject>,
) {
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

/// Returns the bounded process-wide post recorder without retaining business payloads.
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
    /// Copies initialized descriptor fields before the VM can assume payload ownership.
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

    /// Formats the captured descriptor without dereferencing native addresses.
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
    /// Records the outgoing graph while all descriptor and pointer arrays are live.
    fn capture(
        port: i64,
        root: &DartObject,
        envelopes: &[DartObject],
        frames: &[&[DartObject]],
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;

    thread_local! {
        static GRAPH_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Validates VM reference identities and frame contents without taking buffer ownership.
    unsafe fn validate_graph(root: *mut DartObject) -> Result<(), String> {
        let mut pending = vec![root];
        let mut addresses = HashSet::new();
        let mut identities = HashSet::new();
        while let Some(pointer) = pending.pop() {
            let address = pointer as usize;
            if !addresses.insert(address) || !identities.insert(address as u32) {
                return Err("Distinct descriptors share a VM reference identity".into());
            }
            match (*pointer).kind {
                6 => {
                    let array = (*pointer).value.array;
                    pending.extend_from_slice(std::slice::from_raw_parts(
                        array.values,
                        array.length as usize,
                    ));
                }
                3 => {}
                13 if (*pointer).value.external_typed_data.kind == 2 => {}
                kind => return Err(format!("Unexpected descriptor type: {kind}")),
            }
        }
        let min = *addresses.iter().min().unwrap();
        let max = *addresses.iter().max().unwrap();
        if max - min > u32::MAX as usize {
            return Err("Descriptor graph has no bounded 32-bit address span".into());
        }
        let batch = (*root).value.array;
        for index in 0..batch.length as usize {
            let frame = (**batch.values.add(index)).value.array;
            if frame.length as usize != 1 + 4 * (index % 3) {
                return Err(format!("Invalid frame slot count at {index}"));
            }
            let metadata = (**frame.values).value.external_typed_data;
            let bytes = std::slice::from_raw_parts(metadata.data, metadata.length as usize);
            if bytes.len() != 309 || bytes[0] != 1 || bytes[1..9] != (index as i64).to_le_bytes() {
                return Err(format!("Invalid metadata at {index}"));
            }
        }
        Ok(())
    }

    /// Rejects the validated post so the sender must release every external buffer itself.
    unsafe extern "C" fn inspect_post(_: i64, root: *mut DartObject) -> bool {
        GRAPH_ERROR.with(|error| *error.borrow_mut() = validate_graph(root).err());
        false
    }

    /// Checks complete graphs at production batch sizes, including roots and buffer receipts.
    #[test]
    fn batch_descriptors_preserve_compressed_reference_identity() {
        for count in [1, 12, 128] {
            let frames = (0..count)
                .map(|index| {
                    let mut metadata = vec![0; 309];
                    metadata[0] = 1;
                    metadata[1..9].copy_from_slice(&(index as i64).to_le_bytes());
                    let buffers = (0..index % 3)
                        .map(|size| Bytes::from(vec![index as u8; size]))
                        .collect();
                    (metadata, buffers)
                })
                .collect();
            assert!(!DartPort::new(inspect_post, 1).sendBatch(frames));
            GRAPH_ERROR.with(|error| assert_eq!(*error.borrow(), None));
        }
    }
}
