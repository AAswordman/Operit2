//! Exercises production FFI framing, scheduling, stream ownership and VM delivery.
//! Only the application Core is replaced with a deterministic binary echo source.
#![allow(non_snake_case, dead_code)]

#[path = "../src/BridgeCodec.rs"]
mod BridgeCodec;
#[path = "../src/BridgeTransport.rs"]
mod BridgeTransport;
#[path = "../src/FfiTransport.rs"]
mod FfiTransport;

use async_trait::async_trait;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost;
use operit_link::*;
use serde::Deserialize;
use std::ffi::{c_char, CString};
use std::sync::Arc;
use std::time::Duration;

mod BridgeExports {
    /// Preserves the original panic message at the production reply boundary.
    pub(crate) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = payload.downcast_ref::<&str>() {
            message.to_string()
        } else {
            "Non-string fixture panic".into()
        }
    }
}

pub struct OperitFlutterBridge {
    localCore: Arc<ReceiverSource>,
}

impl OperitFlutterBridge {
    /// Uses the production multithreaded Host scheduler for requests and delivery.
    fn runtimeTaskScheduler(&self) -> Arc<dyn HostRuntimeTaskSchedulerHost> {
        Arc::new(NativeHostRuntimeTaskSchedulerHost::new())
    }

    /// Replaces application dispatch while retaining the production transport.
    fn prepareCall(&self, request: CoreCallRequest) -> PreparedCall {
        PreparedCall(self.localCore.clone(), request)
    }
}

struct PreparedCall(Arc<ReceiverSource>, CoreCallRequest);

impl PreparedCall {
    /// Executes one source call through the same interface as the real Core.
    async fn execute(self) -> CoreCallResponse {
        self.0.call(self.1).await
    }
}

struct ReceiverSource;

#[derive(Deserialize)]
struct WatchInput {
    count: usize,
    seed: usize,
    size: usize,
}

/// Builds recognizable binary aliases, slices and zero-length allocations.
fn value(seed: usize, size: usize) -> CoreValue {
    let bytes = CoreBytes::from(vec![(seed % 251) as u8; size]);
    let slice = bytes.slice(size / 2..);
    CoreValue::List(vec![
        CoreValue::Unsigned(seed as u64),
        CoreValue::Bytes(bytes.clone()),
        CoreValue::Bytes(bytes),
        CoreValue::Bytes(slice),
        CoreValue::Bytes(CoreBytes::new()),
    ])
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for ReceiverSource {
    /// Produces out-of-order normal replies and ordinary business errors.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        tokio::task::yield_now().await;
        if request.methodName == "error" {
            return CoreCallResponse::err(
                request.requestId,
                CoreLinkError::new("FIXTURE_ERROR", "Ordinary source failure"),
            );
        }
        CoreCallResponse::ok(request.requestId, request.args)
    }

    /// Encodes snapshot replies through the production event serializer.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        Ok(CoreEvent {
            requestId: Some(request.requestId),
            target: request.target,
            propertyName: request.propertyName,
            kind: CoreEventKind::Snapshot,
            value: request.args,
        })
    }

    /// Rejects unused push operations without changing the connection lifetime.
    async fn openPush(&self, _: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(CoreLinkError::new("FIXTURE_ERROR", "Push is outside this fixture"))
    }

    /// Interleaves finite binary sources with slow openings and cancellation.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.propertyName == "slow" {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        if request.propertyName == "error" {
            return Err(CoreLinkError::new("FIXTURE_ERROR", "Ordinary watch failure"));
        }
        if request.propertyName == "panic" {
            panic!("Watch opening panicked");
        }
        let input: WatchInput = fromCoreValue(request.args)
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        let (sender, stream) = CoreEventStream::channel();
        // Fill a bounded test burst before returning so early completion may race the reply.
        for index in 0..input.count {
            sender.send(CoreEvent {
                requestId: Some(request.requestId.clone()),
                target: request.target.clone(),
                propertyName: request.propertyName.clone(),
                kind: if index + 1 == input.count {
                    CoreEventKind::Completed
                } else {
                    CoreEventKind::Changed
                },
                value: value(input.seed + index, input.size),
            }).map_err(|error| CoreLinkError::internal(error.to_string()))?;
        }
        Ok(stream)
    }
}

/// Opens an independent production ABI-v4 session without booting application services.
#[no_mangle]
pub unsafe extern "C" fn ffi_receiver_connect() -> *mut c_char {
    let bridge = Arc::new(OperitFlutterBridge { localCore: Arc::new(ReceiverSource) });
    FfiTransport::operit_flutter_bridge_ffi_connect(Arc::as_ptr(&bridge))
}

/// Releases only the setup descriptor after Dart has read its function table.
#[no_mangle]
pub unsafe extern "C" fn ffi_receiver_free_descriptor(pointer: *mut c_char) {
    drop(CString::from_raw(pointer));
}

/// Binds the real native logger to an isolated VFS root for persistence assertions.
#[no_mangle]
pub unsafe extern "C" fn ffi_receiver_configure_logs(root: *const c_char) {
    let root = std::path::PathBuf::from(std::ffi::CStr::from_ptr(root).to_str().unwrap());
    let filesystem = operit_vfs::VisualFileSystem::VisualFileSystem::builtinOnly(
        Arc::new(operit_host_native_filesystem::PosixFileSystemHost::new()),
        root.clone(),
        root.join("workspace"),
    );
    operit_util::AppLogger::AppLogger::configure_log_files(
        Arc::new(filesystem),
        "/app/data/ffi.log".into(),
        "/app/data/packages.log".into(),
    ).unwrap();
}
