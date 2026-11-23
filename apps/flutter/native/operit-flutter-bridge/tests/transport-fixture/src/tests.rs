use super::*;
use async_trait::async_trait;
use operit_host_native_common::NativeHostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError, CoreLinkPushSession, CorePushRequest,
    CoreValue, CoreWatchRequest,
};
use std::collections::HashMap;
use std::sync::{mpsc as blocking, Mutex};
use tokio::sync::{mpsc, oneshot};

mod performance;

type Gate = (oneshot::Sender<()>, blocking::Receiver<()>);

#[derive(Default)]
struct ScriptedCore {
    call: Mutex<Option<Gate>>,
    streams: Mutex<HashMap<String, CoreEventStream>>,
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for ScriptedCore {
    /// Injects a genuinely blocking call while ordinary calls return immediately.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        if request.methodName == "block" {
            let (entered, released) = self.call.lock().unwrap().take().unwrap();
            entered.send(()).unwrap();
            let _ = released.recv();
        }
        CoreCallResponse::ok(request.requestId, request.args)
    }

    /// Keeps snapshot operations outside this scripted scenario.
    async fn watchSnapshot(&self, _: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        unreachable!()
    }

    /// Transfers each independently controlled source into the real bridge owner.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        Ok(self
            .streams
            .lock()
            .unwrap()
            .remove(&request.propertyName)
            .unwrap())
    }

    /// Keeps push operations outside this mixed Call/Watch scenario.
    async fn openPush(
        &self,
        _: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        unreachable!()
    }
}

struct Receipt {
    kind: u8,
    tag: i64,
    value: CoreValue,
}

/// Creates a sequenced source event whose order can be checked after transport encoding.
fn event(sequence: u64) -> CoreEvent {
    CoreEvent {
        requestId: None,
        target: "test".into(),
        propertyName: "value".into(),
        kind: CoreEventKind::Changed,
        value: CoreValue::Unsigned(sequence),
    }
}

/// Passes an encoded operation through the actual native connection submission API.
fn submit(session: &Arc<FfiTransport::FfiSession>, operation: u32, tag: i64, bytes: Vec<u8>) {
    let length = bytes.len();
    session.submitRequest(
        operation,
        tag,
        bytes.into_boxed_slice(),
        length,
        Arc::from([]),
    );
}

/// Opens a production watch and requires its encoded acknowledgement to arrive.
async fn open(
    session: &Arc<FfiTransport::FfiSession>,
    received: &mut mpsc::UnboundedReceiver<Receipt>,
    tag: i64,
) {
    let id = format!("stream-{tag}");
    submit(
        session,
        5,
        tag,
        operit_link::encodeLink((&id, &id, "test", &id, CoreValue::Null)).unwrap(),
    );
    let ack = received.recv().await.unwrap();
    assert_eq!((ack.kind, ack.tag), (0, tag));
    assert_eq!(ack.value, CoreValue::String(id));
}

/// Checks real FFI delivery while a call and a source poll block, including ANR and recovery.
#[tokio::test]
async fn blocked_call_and_slow_stream_leave_eight_streams_and_replies_live() {
    let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = logs.clone();
    operit_host_api::setHostLogSink(Arc::new(move |tag, message| {
        eprintln!("{tag}: {message}");
        captured.lock().unwrap().push(format!("{tag}: {message}"));
    }));
    let (call_entered, call_started) = oneshot::channel();
    let (release_call, call_released) = blocking::channel();
    let core = Arc::new(ScriptedCore::default());
    *core.call.lock().unwrap() = Some((call_entered, call_released));
    let mut healthy = Vec::new();
    for tag in 1..=8 {
        let (sender, stream) = CoreEventStream::channel();
        core.streams
            .lock()
            .unwrap()
            .insert(format!("stream-{tag}"), stream);
        healthy.push(sender);
    }
    let (slow_entered, slow_started) = oneshot::channel();
    let (release_slow, slow_released) = blocking::channel();
    let (slow, values) = mpsc::unbounded_channel();
    let mut slow_entered = Some(slow_entered);
    core.streams.lock().unwrap().insert(
        "stream-50".into(),
        CoreEventStream::fromMappedReceiver(values, move |sequence| {
            if let Some(entered) = slow_entered.take() {
                entered.send(()).unwrap();
                let _ = slow_released.recv();
            }
            Some(event(sequence))
        }),
    );
    let (sporadic, stream) = CoreEventStream::channel();
    core.streams
        .lock()
        .unwrap()
        .insert("stream-51".into(), stream);
    let (delivered, mut received) = mpsc::unbounded_channel();
    let bridge = Arc::new(OperitFlutterBridge {
        localCore: core,
        scheduler: scheduler.clone(),
    });
    let session = FfiTransport::FfiSession::forWire(
        bridge,
        Arc::new(move |frames| {
            for frame in frames {
                let (bytes, buffers) = frame.into_parts();
                let kind = bytes[0];
                let tag = i64::from_le_bytes(bytes[1..9].try_into().unwrap());
                let value = operit_link::codec::withLinkBuffers(buffers.into(), || match kind {
                    0 => {
                        let (status, value): (u8, CoreValue) =
                            operit_link::decodeLink(&bytes[9..]).unwrap();
                        assert_eq!(status, 0);
                        value
                    }
                    1 => {
                        let (_, (_, _, _, _, value)): (
                            String,
                            (Option<String>, String, String, String, CoreValue),
                        ) = operit_link::decodeLink(&bytes[9..]).unwrap();
                        value
                    }
                    3 => CoreValue::Null,
                    _ => panic!("Unexpected delivery kind={kind} tag={tag}"),
                });
                if delivered.send(Receipt { kind, tag, value }).is_err() {
                    return false;
                }
            }
            true
        }),
    )
    .unwrap();
    let scenario = async {
        for tag in (1..=4).chain([50, 51]) {
            open(&session, &mut received, tag).await;
        }
        submit(
            &session,
            0,
            10_000,
            operit_link::encodeLink(("blocked-call", "test", "block", CoreValue::Unsigned(999)))
                .unwrap(),
        );
        call_started.await.unwrap();
        slow.send(0).unwrap();
        slow_started.await.unwrap();
        for tag in 5..=8 {
            open(&session, &mut received, tag).await;
        }
        let started = scheduler.monotonicTimeMillis().unwrap();
        let mut slow_count = 0;
        let mut rounds = 0u64;
        let mut max_round_ms = 0;
        while scheduler.monotonicTimeMillis().unwrap() - started < 2_400 {
            let round_started = scheduler.monotonicTimeMillis().unwrap();
            for sender in &healthy {
                sender.send(event(rounds)).unwrap();
            }
            let sporadic_ready = rounds % 4 == 0;
            if sporadic_ready {
                sporadic.send(event(slow_count)).unwrap();
            }
            submit(
                &session,
                0,
                20_000,
                operit_link::encodeLink(("fast-call", "test", "echo", CoreValue::Unsigned(rounds)))
                    .unwrap(),
            );
            submit(
                &session,
                6,
                30_000,
                operit_link::encodeLink(("absent", 0i64)).unwrap(),
            );
            let mut seen = [false; 8];
            let (mut replied, mut cancelled, mut sporadic_seen) = (false, false, false);
            for _ in 0..(10 + usize::from(sporadic_ready)) {
                let receipt = received.recv().await.unwrap();
                match (receipt.kind, receipt.tag) {
                    (1, tag @ 1..=8) => {
                        assert!(!seen[tag as usize - 1]);
                        seen[tag as usize - 1] = true;
                        assert_eq!(receipt.value, CoreValue::Unsigned(rounds));
                    }
                    (1, 51) => {
                        assert_eq!(receipt.value, CoreValue::Unsigned(slow_count));
                        sporadic_seen = true;
                    }
                    (0, 20_000) => {
                        assert_eq!(receipt.value, CoreValue::Unsigned(rounds));
                        replied = true;
                    }
                    (0, 30_000) => cancelled = true,
                    (kind, tag) => {
                        panic!("Blocked work or unexpected frame arrived: kind={kind} tag={tag}")
                    }
                }
            }
            assert!(seen.into_iter().all(|seen| seen) && replied && cancelled);
            assert_eq!(sporadic_seen, sporadic_ready);
            slow_count += u64::from(sporadic_ready);
            max_round_ms =
                max_round_ms.max(scheduler.monotonicTimeMillis().unwrap() - round_started);
            rounds += 1;
            scheduler.waitForHostRuntimeDelay(20).await.unwrap();
        }
        let before_release = logs.lock().unwrap().clone();
        assert!(before_release.iter().any(|line| line
            .starts_with("HostRuntimeANR: ANR task=operit-connection-call ")
            && line.ends_with("phase=polling")));
        assert!(before_release.iter().any(|line| line
            .starts_with("HostRuntimeANR: ANR task=operit-bridge-watch id=stream-50 ")
            && line.ends_with("phase=polling")));
        assert!(!before_release
            .iter()
            .any(|line| line.starts_with("HostRuntimeANR: ANR task=operit-ffi-delivery ")));
        assert_eq!(
            before_release
                .iter()
                .filter(|line| line.starts_with("HostRuntimeANR:"))
                .count(),
            2
        );
        release_call.send(()).unwrap();
        release_slow.send(()).unwrap();
        let mut recovered = Vec::new();
        for _ in 0..2 {
            let receipt = received.recv().await.unwrap();
            recovered.push((receipt.kind, receipt.tag, receipt.value));
        }
        assert!(recovered.contains(&(0, 10_000, CoreValue::Unsigned(999))));
        assert!(recovered.contains(&(1, 50, CoreValue::Unsigned(0))));
        for tag in 1..=8 {
            submit(
                &session,
                6,
                40_000 + tag,
                operit_link::encodeLink((format!("stream-{tag}"), tag)).unwrap(),
            );
        }
        let mut closed = 0;
        let mut acknowledgements = 0;
        for _ in 0..16 {
            let receipt = received.recv().await.unwrap();
            match receipt.kind {
                0 => acknowledgements += 1,
                3 => closed += 1,
                _ => panic!("Unexpected close frame"),
            }
        }
        assert_eq!((acknowledgements, closed), (8, 8));
        println!("mixed transport: healthyFrames={} sporadicFrames={slow_count} fastCalls={rounds} controlReplies={rounds} maxRoundMs={max_round_ms}; blocked call + blocked source both reported ANR before release; recovery and 8 closes passed", rounds * 8);
        session.close();
    };
    tokio::select! {
        _ = scheduler.waitForHostRuntimeDelay(10_000) => panic!("Mixed transport scenario stalled"),
        _ = scenario => {},
    }
}
