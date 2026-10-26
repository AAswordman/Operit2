//! Fixed input source for the production native -> Dart Markdown pipeline.
//! Bridge framing, stream ownership, scheduler, VM delivery and finalizers are
//! included from production, not reimplemented by this benchmark.
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
use operit_util::MarkdownRenderStream::MarkdownRenderEventStream;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, CString};
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod BridgeExports {
    pub(crate) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = payload.downcast_ref::<&str>() {
            message.to_string()
        } else {
            "Native benchmark panic".into()
        }
    }
}

// Only the application source is replaced. The production FfiTransport module
// uses this small host in place of bootstrapping accounts/providers/storage.
pub struct OperitFlutterBridge {
    localCore: Arc<MarkdownSource>,
}
impl OperitFlutterBridge {
    fn runtimeTaskScheduler(&self) -> Arc<dyn HostRuntimeTaskSchedulerHost> {
        Arc::new(NativeHostRuntimeTaskSchedulerHost::new())
    }
    fn prepareCall(&self, request: CoreCallRequest) -> PreparedCall {
        PreparedCall(self.localCore.clone(), request)
    }
}
struct PreparedCall(Arc<MarkdownSource>, CoreCallRequest);
impl PreparedCall {
    async fn execute(self) -> CoreCallResponse {
        self.0.call(self.1).await
    }
}

#[derive(Default)]
struct MarkdownSource {
    metrics: Arc<Mutex<Metrics>>,
}
#[derive(Default, Serialize)]
struct Metrics {
    parseMicros: u64,
    valueMicros: u64,
    sourceMicros: u64,
    events: usize,
    chunks: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    source: String,
    chunk_chars: usize,
    instrument: bool,
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for MarkdownSource {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        if request.methodName == "splitMarkdownContent" {
            #[derive(Deserialize)]
            struct StaticInput {
                content: String,
            }
            let result = fromCoreValue::<StaticInput>(request.args)
                .and_then(|input| {
                    toCoreValue(MarkdownRenderEventStream::fromContent(input.content))
                })
                .map_err(|error| CoreLinkError::internal(error.to_string()));
            return CoreCallResponse {
                requestId: request.requestId,
                result,
            };
        }
        CoreCallResponse::ok(
            request.requestId,
            toCoreValue(&*self.metrics.lock().unwrap()).unwrap(),
        )
    }
    async fn watchSnapshot(&self, _: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        Err(CoreLinkError::new("BENCH", "Use the streaming benchmark"))
    }
    async fn openPush(
        &self,
        _: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(CoreLinkError::new("BENCH", "Use the streaming benchmark"))
    }
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let input: Input = fromCoreValue(request.args)
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        if input.chunk_chars == 0 {
            return Err(CoreLinkError::new("BENCH", "chunkChars must be positive"));
        }
        // Split input outside the measured source region, preserving UTF-8.
        let mut offsets: Vec<usize> = input
            .source
            .char_indices()
            .enumerate()
            .filter(|(i, _)| i % input.chunk_chars == 0)
            .map(|(_, (offset, _))| offset)
            .collect();
        offsets.push(input.source.len());
        let metrics = self.metrics.clone();
        let (sender, stream) = CoreEventStream::channel();
        NativeHostRuntimeTaskSchedulerHost::new()
            .scheduleHostRuntimeAsyncTask(
                "markdown-benchmark-source",
                Box::new(move || {
                    Box::pin(async move {
                        let started = Instant::now();
                        let mut parser = MarkdownRenderEventStream::new("benchmark-chat".into());
                        let mut measured = Metrics::default();
                        let mut parse_nanos = 0;
                        let mut value_nanos = 0;
                        // Match MessageProcessingDelegate: one CoreEvent per semantic
                        // event, unbounded source channel, only root EOF is Completed.
                        for chunk in offsets
                            .windows(2)
                            .map(|range| &input.source[range[0]..range[1]])
                        {
                            let start = input.instrument.then(Instant::now);
                            let events = parser.pushChunk(chunk);
                            if let Some(start) = start {
                                parse_nanos += start.elapsed().as_nanos();
                            }
                            measured.chunks += 1;
                            for event in events {
                                let start = input.instrument.then(Instant::now);
                                let value = toCoreValue(event).unwrap();
                                if let Some(start) = start {
                                    value_nanos += start.elapsed().as_nanos();
                                }
                                measured.events += 1;
                                if sender
                                    .send(CoreEvent {
                                        requestId: Some(request.requestId.clone()),
                                        target: request.target.clone(),
                                        propertyName: request.propertyName.clone(),
                                        kind: CoreEventKind::Changed,
                                        value,
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                            }
                        }
                        let start = input.instrument.then(Instant::now);
                        let events = parser.finish();
                        if let Some(start) = start {
                            parse_nanos += start.elapsed().as_nanos();
                        }
                        // Publish metrics before EOF can be observed on the Dart side.
                        let mut final_event = None;
                        for event in events {
                            let kind = if event.isRootCompletion() {
                                CoreEventKind::Completed
                            } else {
                                CoreEventKind::Changed
                            };
                            let start = input.instrument.then(Instant::now);
                            let value = toCoreValue(event).unwrap();
                            if let Some(start) = start {
                                value_nanos += start.elapsed().as_nanos();
                            }
                            measured.events += 1;
                            let event = CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                target: request.target.clone(),
                                propertyName: request.propertyName.clone(),
                                kind,
                                value,
                            };
                            if event.kind == CoreEventKind::Completed {
                                final_event = Some(event);
                            } else if sender.send(event).is_err() {
                                return;
                            }
                        }
                        measured.parseMicros = (parse_nanos / 1000) as u64;
                        measured.valueMicros = (value_nanos / 1000) as u64;
                        measured.sourceMicros = started.elapsed().as_micros() as u64;
                        *metrics.lock().unwrap() = measured;
                        if let Some(event) = final_event {
                            let _ = sender.send(event);
                        }
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(stream)
    }
}

/// Supplies the real ABI-v3 descriptor; JSON here is connection setup metadata,
/// never Markdown event encoding, and setup is excluded from measurements.
#[no_mangle]
pub unsafe extern "C" fn markdown_bench_connect() -> *mut c_char {
    let bridge = Arc::new(OperitFlutterBridge {
        localCore: Arc::new(MarkdownSource::default()),
    });
    FfiTransport::operit_flutter_bridge_ffi_connect(Arc::as_ptr(&bridge))
}
#[no_mangle]
pub unsafe extern "C" fn markdown_bench_free_descriptor(pointer: *mut c_char) {
    drop(CString::from_raw(pointer));
}
