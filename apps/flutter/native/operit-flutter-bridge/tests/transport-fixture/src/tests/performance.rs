//! Measures bounded concurrent source traffic through production encoding and FFI batching.
use super::*;
use tokio::sync::Semaphore;

enum Payload {
    Text(String),
    Binary(operit_link::CoreBytes),
}

impl Payload {
    /// Allocates text per message while retaining binary attachments through their shared handle.
    fn value(&self) -> CoreValue {
        match self {
            Self::Text(text) => CoreValue::String(text.clone()),
            Self::Binary(bytes) => CoreValue::Bytes(bytes.clone()),
        }
    }
}

#[derive(Default)]
struct Metrics {
    next: Vec<u64>,
    latency_ms: Vec<u64>,
    call_latency_ms: Vec<u64>,
    batches: u64,
    max_batch: usize,
    encoded_bytes: u64,
    attachment_bytes: u64,
    finished_ms: u64,
    completed: Option<oneshot::Sender<()>>,
}

/// Returns an observed percentile without interpolating below a measured sample.
fn percentile(values: &mut [u64], percent: usize) -> u64 {
    values.sort_unstable();
    values[(values.len() * percent).div_ceil(100).saturating_sub(1)]
}

/// Measures one concurrent load with sixteen outstanding events per source.
async fn measure(
    stream_count: usize,
    size: usize,
    binary: bool,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
) {
    let per_stream = if size == 1024 || binary {
        500_000 / stream_count
    } else {
        32_768 / stream_count
    };
    let total = per_stream * stream_count;
    let core = Arc::new(ScriptedCore::default());
    let windows: Arc<Vec<_>> = Arc::new(
        (0..stream_count)
            .map(|_| Arc::new(Semaphore::new(16)))
            .collect(),
    );
    let mut senders = Vec::new();
    for index in 0..stream_count {
        let (sender, stream) = CoreEventStream::boundedChannel(64);
        core.streams
            .lock()
            .unwrap()
            .insert(format!("stream-{}", index + 1), stream);
        senders.push(sender);
    }
    let (completed, mut done) = oneshot::channel();
    let metrics = Arc::new(Mutex::new(Metrics {
        next: vec![0; stream_count],
        completed: Some(completed),
        ..Metrics::default()
    }));
    let observed = metrics.clone();
    let released = windows.clone();
    let clock = scheduler.clone();
    let (acknowledged, mut acknowledgements) = mpsc::unbounded_channel();
    let bridge = Arc::new(OperitFlutterBridge {
        localCore: core,
        scheduler: scheduler.clone(),
    });
    let session = FfiTransport::FfiSession::forWire(
        bridge,
        Arc::new(move |frames| {
            let received_at = clock.monotonicTimeMillis().unwrap();
            let mut stats = observed.lock().unwrap();
            let mut batch_events = 0;
            for frame in frames {
                let (bytes, buffers) = frame.into_parts();
                let tag = i64::from_le_bytes(bytes[1..9].try_into().unwrap());
                stats.encoded_bytes += bytes.len() as u64;
                stats.attachment_bytes +=
                    buffers.iter().map(|bytes| bytes.len() as u64).sum::<u64>();
                operit_link::codec::withLinkBuffers(buffers.into(), || match bytes[0] {
                    0 => {
                        let (status, value): (u8, CoreValue) =
                            operit_link::decodeLink(&bytes[9..]).unwrap();
                        assert_eq!(status, 0);
                        if tag == 500_000 {
                            let CoreValue::Unsigned(started) = value else {
                                panic!("Missing call timestamp")
                            };
                            stats.call_latency_ms.push(received_at - started);
                        } else {
                            acknowledged.send(tag).unwrap();
                        }
                    }
                    1 => {
                        let (_, (_, _, _, _, value)): (
                            String,
                            (Option<String>, String, String, String, CoreValue),
                        ) = operit_link::decodeLink(&bytes[9..]).unwrap();
                        let CoreValue::List(fields) = value else {
                            panic!("Missing event fields")
                        };
                        let [CoreValue::Unsigned(sequence), CoreValue::Unsigned(started), payload] =
                            fields.as_slice()
                        else {
                            panic!("Invalid event fields")
                        };
                        let index = tag as usize - 1;
                        assert_eq!(
                            *sequence, stats.next[index],
                            "source lost, duplicated or reordered an event"
                        );
                        match payload {
                            CoreValue::String(text) if !binary => assert_eq!(text.len(), size),
                            CoreValue::Bytes(bytes) if binary => assert_eq!(bytes.len(), size),
                            _ => panic!("Unexpected payload kind"),
                        }
                        stats.next[index] += 1;
                        stats.latency_ms.push(received_at - started);
                        released[index].add_permits(1);
                        batch_events += 1;
                    }
                    3 => {}
                    kind => panic!("Unexpected frame kind={kind}"),
                });
            }
            if batch_events > 0 {
                stats.batches += 1;
                stats.max_batch = stats.max_batch.max(batch_events);
            }
            if stats.latency_ms.len() == total {
                if let Some(completed) = stats.completed.take() {
                    stats.finished_ms = clock.monotonicTimeMillis().unwrap();
                    completed.send(()).unwrap();
                }
            }
            true
        }),
    )
    .unwrap();
    for tag in 1..=stream_count as i64 {
        let id = format!("stream-{tag}");
        submit(
            &session,
            5,
            tag,
            operit_link::encodeLink((&id, &id, "test", &id, CoreValue::Null)).unwrap(),
        );
        assert_eq!(acknowledgements.recv().await.unwrap(), tag);
    }
    let payload = Arc::new(if binary {
        Payload::Binary(vec![0x5a; size].into())
    } else {
        Payload::Text("x".repeat(size))
    });
    let started = scheduler.monotonicTimeMillis().unwrap();
    for (index, sender) in senders.into_iter().enumerate() {
        let payload = payload.clone();
        let clock = scheduler.clone();
        let window = windows[index].clone();
        scheduler
            .scheduleHostRuntimePooledTask(
                "transport-throughput-producer",
                None,
                Box::pin(async move {
                    for sequence in 0..per_stream {
                        window.acquire().await.unwrap().forget();
                        let mut value = event(sequence as u64);
                        value.value = CoreValue::List(vec![
                            CoreValue::Unsigned(sequence as u64),
                            CoreValue::Unsigned(clock.monotonicTimeMillis().unwrap()),
                            payload.value(),
                        ]);
                        sender.send(value).await.unwrap();
                    }
                }),
            )
            .unwrap();
    }
    loop {
        tokio::select! {
            result = &mut done => { result.unwrap(); break; },
            _ = scheduler.waitForHostRuntimeDelay(10) => {
                submit(&session, 0, 500_000, operit_link::encodeLink(("latency", "test", "echo",
                    CoreValue::Unsigned(scheduler.monotonicTimeMillis().unwrap()))).unwrap());
            }
        }
    }
    let mut stats = metrics.lock().unwrap();
    assert!(stats.next.iter().all(|count| *count == per_stream as u64));
    let elapsed = stats.finished_ms - started;
    assert!(elapsed > 0);
    let p95 = percentile(&mut stats.latency_ms, 95);
    let p99 = percentile(&mut stats.latency_ms, 99);
    let call_p99 = if stats.call_latency_ms.is_empty() {
        0
    } else {
        percentile(&mut stats.call_latency_ms, 99)
    };
    println!("PERF streams={stream_count} payload={} bytes={size} frames={total} elapsedMs={elapsed} eventsPerSec={:.0} payloadMiBPerSec={:.1} p95Ms={p95} p99Ms={p99} callSamples={} callP99Ms={call_p99} avgBatch={:.1} maxBatch={} encodedMiB={:.1} attachmentMiB={:.1}",
        if binary { "binary" } else { "text" }, total as f64 * 1000.0 / elapsed as f64,
        total as f64 * size as f64 * 1000.0 / elapsed as f64 / 1_048_576.0,
        stats.call_latency_ms.len(), total as f64 / stats.batches as f64, stats.max_batch,
        stats.encoded_bytes as f64 / 1_048_576.0, stats.attachment_bytes as f64 / 1_048_576.0);
    drop(stats);
    session.close();
}

/// Runs optimized text and shared-binary load cases while checking every stream's sequence.
#[tokio::test]
#[ignore = "run explicitly with --release --ignored --nocapture"]
async fn concurrent_stream_throughput() {
    operit_host_api::setHostLogSink(Arc::new(|tag, message| eprintln!("{tag}: {message}")));
    let scheduler: Arc<dyn HostRuntimeTaskSchedulerHost> =
        Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    let scenario = async {
        for (size, binary) in [(1024, false), (65536, false), (65536, true)] {
            for streams in [1, 8, 32] {
                measure(streams, size, binary, scheduler.clone()).await;
            }
        }
    };
    tokio::select! {
        _ = scheduler.waitForHostRuntimeDelay(120_000) => panic!("Throughput benchmark stalled"),
        _ = scenario => {},
    }
}
