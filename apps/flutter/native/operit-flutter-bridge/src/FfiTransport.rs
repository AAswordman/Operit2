//! Direct Dart transport with one connection-owned Push/Watch session.
//! ABI v3 delivers batches through a single VM port with shared binary ownership.

use crate::BridgeCodec::*;
use crate::BridgeExports;
use crate::BridgeTransport::{BridgeStreams, StreamAction, StreamReply, WATCH_BATCH_CAPACITY};
use crate::OperitFlutterBridge;
use futures_util::FutureExt;
use operit_host_native_common::DartPort::{DartPort, PostCObject};
use operit_host_native_common::SharedBuffer::{
    shared_allocate, shared_data, shared_release, shared_retain, shared_seal, SharedBuffer,
};
use operit_link::{CoreLinkError, CoreLinkSharedClient};
use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub struct FfiSession {
    bridge: Arc<OperitFlutterBridge>,
    port: OnceLock<DartPort>,
    delivery: Mutex<()>,
    streams: OnceLock<BridgeStreams>,
    closed: AtomicBool,
}

impl FfiSession {
    /// Delivers one ordinary response using the same batch envelope as watch events.
    fn post(&self, frame: NativeFrame) {
        self.postBatch(vec![frame]);
    }

    /// Transfers ready frames together while preserving each frame's binary attachment scope.
    fn postBatch(&self, frames: Vec<NativeFrame>) {
        let delivery = self.delivery.lock().expect("FFI delivery lock");
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        if !self
            .port
            .get()
            .expect("FFI session must be connected")
            .sendBatch(frames.into_iter().map(NativeFrame::into_parts).collect())
        {
            drop(delivery);
            self.close();
        }
    }

    /// Cancels every source and pending stream operation belonging to this connection.
    fn close(&self) {
        let _delivery = self.delivery.lock().expect("FFI delivery lock");
        self.closed.store(true, Ordering::Release);
        if let Some(streams) = self.streams.get() {
            streams.close();
        }
    }

    /// Starts one multiplexing owner and one batched delivery task for this Dart connection.
    fn attach(self: &Arc<Self>, post: PostCObject, port: i64) -> bool {
        if self.port.set(DartPort::new(post, port)).is_err() {
            return false;
        }
        let scheduler = self.bridge.runtimeTaskScheduler();
        let (streams, mut events) =
            match BridgeStreams::start(self.bridge.localCore.clone(), scheduler.as_ref()) {
                Ok(connection) => connection,
                Err(_) => return false,
            };
        if self.streams.set(streams).is_err() {
            return false;
        }
        let weak = Arc::downgrade(self);
        let result = scheduler.scheduleHostRuntimeAsyncTask(
            "operit-ffi-stream-delivery",
            Box::new(move || {
                Box::pin(async move {
                    let mut batch = Vec::with_capacity(WATCH_BATCH_CAPACITY);
                    while events.recv_many(&mut batch, WATCH_BATCH_CAPACITY).await != 0 {
                        let Some(session) = weak.upgrade() else {
                            return;
                        };
                        session.postBatch(
                            batch
                                .drain(..)
                                .flat_map(|delivery| match delivery.event {
                                    Ok(Some(event)) => {
                                        vec![NativeFrame::watch(delivery.tag, &delivery.id, event)]
                                    }
                                    Ok(None) => vec![NativeFrame::empty(3, delivery.tag)],
                                    Err(error) => vec![
                                        NativeFrame::result(2, delivery.tag, Err::<(), _>(error)),
                                        NativeFrame::empty(3, delivery.tag),
                                    ],
                                })
                                .collect(),
                        );
                    }
                })
            }),
        );
        if result.is_err() {
            self.close();
            return false;
        }
        true
    }

    /// Decodes and enqueues stream commands in Dart submission order before scheduling replies.
    fn submitStream(
        &self,
        operation: u32,
        request: i64,
        bytes: &[u8],
        buffers: Arc<[operit_link::CoreBytes]>,
    ) -> Result<StreamReply, CoreLinkError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(CoreLinkError::new("FFI_CLOSED", "FFI connection is closed"));
        }
        let action = operit_link::codec::withLinkBuffers(buffers, || match operation {
            1 => decode_native_push_open_request(bytes).map(StreamAction::OpenPush),
            2 => decode_native_push_item(bytes).map(StreamAction::PushItem),
            3 => std::str::from_utf8(bytes)
                .map_err(|error| CoreLinkError::internal(error.to_string()))
                .map(|id| StreamAction::ClosePush(id.to_string())),
            6 => operit_link::decodeLink::<(String, i64)>(bytes)
                .map(|(id, tag)| StreamAction::CloseWatch { id, tag })
                .map_err(|error| CoreLinkError::internal(error.to_string())),
            5 => decode_native_watch_stream_request(bytes).map(|(id, watch)| {
                StreamAction::OpenWatch {
                    id,
                    request: watch,
                    tag: request,
                }
            }),
            _ => Err(CoreLinkError::new(
                "FFI_OPERATION",
                "Unknown stream operation",
            )),
        })?;
        self.streams
            .get()
            .expect("FFI stream session must be attached")
            .submit(action)
    }

    /// Executes independent calls and snapshots without blocking the stream owner's mailbox.
    async fn dispatch(
        &self,
        operation: u32,
        request: i64,
        bytes: &[u8],
        buffers: Arc<[operit_link::CoreBytes]>,
    ) -> NativeFrame {
        if self.closed.load(Ordering::Acquire) {
            return NativeFrame::error(0, request, "FFI_CLOSED", "FFI connection is closed");
        }
        match operation {
            0 => {
                let result = match operit_link::codec::withLinkBuffers(buffers, || {
                    decode_native_call_request(bytes)
                }) {
                    Ok(request) => self.bridge.prepareCall(request).execute().await.result,
                    Err(error) => Err(error),
                };
                NativeFrame::result(0, request, result)
            }
            4 => {
                let result = match operit_link::codec::withLinkBuffers(buffers, || {
                    decode_native_watch_snapshot_request(bytes)
                }) {
                    Ok(request) => {
                        CoreLinkSharedClient::watchSnapshot(self.bridge.localCore.as_ref(), request)
                            .await
                    }
                    Err(error) => Err(error),
                };
                NativeFrame::result(0, request, result.map(native_watch_event_payload))
            }
            _ => NativeFrame::error(0, request, "FFI_OPERATION", "Unknown FFI operation"),
        }
    }
}

impl Drop for FfiSession {
    /// Releases the connection's stream owner while preserving the platform Runtime.
    fn drop(&mut self) {
        self.close();
    }
}

/// Creates a retained ABI-v3 connection from the live runtime owned by the platform host.
#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_ffi_connect(
    handle: *const OperitFlutterBridge,
) -> *mut c_char {
    assert!(!handle.is_null(), "FFI requires a live host runtime");
    Arc::increment_strong_count(handle);
    let session = Arc::new(FfiSession {
        bridge: Arc::from_raw(handle),
        port: OnceLock::new(),
        delivery: Mutex::new(()),
        streams: OnceLock::new(),
        closed: AtomicBool::new(false),
    });
    let descriptor = serde_json::json!({
        "version": 3,
        // Pointer addresses must remain strings across JSON's numeric precision boundary.
        "session": (Arc::into_raw(session) as usize).to_string(),
        "attach": (ffi_attach as *const () as usize).to_string(),
        "submit": (ffi_submit as *const () as usize).to_string(),
        "allocate": (ffi_allocate as *const () as usize).to_string(),
        "bufferAllocate": (shared_allocate as *const () as usize).to_string(),
        "bufferData": (shared_data as *const () as usize).to_string(),
        "bufferSeal": (shared_seal as *const () as usize).to_string(),
        "bufferRelease": (shared_release as *const () as usize).to_string(),
        "bufferRetain": (shared_retain as *const () as usize).to_string(),
        "free": (ffi_free as *const () as usize).to_string(),
        "release": (ffi_release as *const () as usize).to_string(),
    });
    CString::new(descriptor.to_string())
        .expect("FFI descriptor JSON")
        .into_raw()
}

/// Installs the VM port and initializes this connection's shared stream owner exactly once.
unsafe extern "C" fn ffi_attach(pointer: *const FfiSession, post: PostCObject, port: i64) -> bool {
    Arc::increment_strong_count(pointer);
    Arc::from_raw(pointer).attach(post, port)
}

/// Allocates request storage that Dart completely initializes before submitting it.
unsafe extern "C" fn ffi_allocate(length: usize) -> *mut u8 {
    Box::into_raw(Box::<[u8]>::new_uninit_slice(length)).cast::<u8>()
}

/// Releases exactly one buffer allocated by the connection's allocator.
unsafe extern "C" fn ffi_free(pointer: *mut u8, length: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
        pointer.cast::<std::mem::MaybeUninit<u8>>(),
        length,
    )));
}

/// Preserves stream ingress order while executing calls and awaiting replies on the Host scheduler.
unsafe extern "C" fn ffi_submit(
    pointer: *const FfiSession,
    operation: u32,
    request: i64,
    bytes: *mut u8,
    length: usize,
    handles: *const *const SharedBuffer,
    handle_count: usize,
) {
    Arc::increment_strong_count(pointer);
    let session = Arc::from_raw(pointer);
    let width = std::mem::size_of::<usize>();
    let allocation_length = (length + width - 1) / width * width + handle_count * width;
    let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, allocation_length));
    let buffers: Arc<[operit_link::CoreBytes]> = (0..handle_count)
        .map(|index| SharedBuffer::retain(*handles.add(index)))
        .collect();
    let stream = match operation {
        1 | 2 | 3 | 5 | 6 => {
            Some(session.submitStream(operation, request, &bytes[..length], buffers.clone()))
        }
        _ => None,
    };
    let worker_session = session.clone();
    let result = session
        .bridge
        .runtimeTaskScheduler()
        .scheduleHostRuntimeAsyncTask(
            "operit-ffi-request",
            Box::new(move || {
                Box::pin(async move {
                    let response = std::panic::AssertUnwindSafe(async {
                        match stream {
                            Some(reply) => {
                                let result = match reply {
                                    Ok(reply) => reply.await.map_err(|_| {
                                        CoreLinkError::new(
                                            "STREAM_CLOSED",
                                            "Connection stream owner stopped",
                                        )
                                    })?,
                                    Err(error) => Err(error),
                                };
                                Ok(NativeFrame::result(0, request, result))
                            }
                            None => Ok(worker_session
                                .dispatch(operation, request, &bytes[..length], buffers)
                                .await),
                        }
                    })
                    .catch_unwind()
                    .await;
                    let response = match response {
                        Ok(Ok(frame)) => frame,
                        Ok(Err(error)) => {
                            NativeFrame::result(0, request, Err::<(), CoreLinkError>(error))
                        }
                        Err(payload) => NativeFrame::error(
                            0,
                            request,
                            "FATAL_CORE_PANIC",
                            BridgeExports::panic_payload_message(payload.as_ref()),
                        ),
                    };
                    worker_session.post(response);
                })
            }),
        );
    if let Err(error) = result {
        session.post(NativeFrame::error(
            0,
            request,
            "FFI_SCHEDULE",
            error.to_string(),
        ));
    }
}

/// Detaches Dart and cancels all streams without destroying the platform runtime.
unsafe extern "C" fn ffi_release(pointer: *mut c_void) {
    let session = Arc::from_raw(pointer.cast::<FfiSession>());
    session.close();
}
