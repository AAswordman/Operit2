//! Orders configured speech on the same host used by application playback.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tokio::sync::oneshot;

use operit_host_api::{HostRuntimeTaskSchedulerHost, TtsPlaybackHost, TtsPlaybackStatus};
use operit_util::AppLogger::AppLogger;

pub(super) type SpeechStart =
    Box<dyn FnOnce(&dyn TtsPlaybackHost) -> Result<TtsPlaybackStatus, String> + Send>;

const CANCELLED: &str = "TTS playback was cancelled";

type PrepareSpeech = Box<dyn FnOnce() -> Result<Vec<SpeechStart>, String> + Send>;

type QueueRegistry = Vec<(Weak<dyn TtsPlaybackHost>, Weak<SpeechQueue>)>;

#[derive(Default)]
struct QueueState {
    next: u64,
    pending: VecDeque<SpeechEntry>,
    playing: Option<(u64, String)>,
}

struct SpeechEntry {
    ticket: u64,
    key: Option<(String, String)>,
}

#[derive(Default)]
struct SpeechQueue(Mutex<QueueState>);

fn queue_for(host: &Arc<dyn TtsPlaybackHost>) -> Arc<SpeechQueue> {
    static QUEUES: OnceLock<Mutex<QueueRegistry>> = OnceLock::new();
    let mut queues = QUEUES.get_or_init(Default::default).lock().unwrap();
    queues.retain(|(host, queue)| host.strong_count() > 0 && queue.strong_count() > 0);
    for (owner, queue) in queues.iter() {
        if owner.ptr_eq(&Arc::downgrade(host)) {
            if let Some(queue) = queue.upgrade() {
                return queue;
            }
        }
    }
    let queue = Arc::new(SpeechQueue::default());
    queues.push((Arc::downgrade(host), Arc::downgrade(&queue)));
    queue
}

/// Cancels queued continuations atomically with a direct application playback operation.
pub(super) fn replace_playback<T>(
    host: &Arc<dyn TtsPlaybackHost>,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let queue = queue_for(host);
    let mut state = queue.0.lock().map_err(|error| error.to_string())?;
    state.pending.clear();
    state.playing = None;
    operation()
}

/// Removes only the requested plugin's utterance and stops its current segment if still active.
pub(super) fn cancel_configured_speech(
    host: &Arc<dyn TtsPlaybackHost>,
    owner: &str,
    id: &str,
) -> Result<bool, String> {
    let queue = queue_for(host);
    let mut state = queue.0.lock().map_err(|error| error.to_string())?;
    let Some(index) = state.pending.iter().position(|entry| {
        entry.key.as_ref().is_some_and(|(entry_owner, entry_id)| entry_owner == owner && entry_id == id)
    }) else { return Ok(false); };
    let ticket = state.pending[index].ticket;
    if let Some((playing, path)) = state.playing.as_ref() {
        if *playing == ticket {
            let status = host.speechState().map_err(|error| error.to_string())?;
            if status.active && status.path == *path {
                host.stopSpeech().map_err(|error| error.to_string())?;
            }
            state.playing = None;
        }
    }
    state.pending.remove(index);
    Ok(true)
}

impl SpeechQueue {
    fn with_turn<T>(
        &self,
        ticket: u64,
        operation: impl FnOnce(&mut QueueState) -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let mut state = self.0.lock().map_err(|error| error.to_string())?;
        if !state.pending.iter().any(|entry| entry.ticket == ticket) {
            return Err(CANCELLED.into());
        }
        if state.pending.front().map(|entry| entry.ticket) != Some(ticket) {
            return Ok(None);
        }
        operation(&mut state).map(Some)
    }

    fn finish(&self, ticket: u64) {
        if let Ok(mut state) = self.0.lock() {
            state.pending.retain(|entry| entry.ticket != ticket);
            if state.playing.as_ref().is_some_and(|(playing, _)| *playing == ticket) {
                state.playing = None;
            }
        }
    }
}

/// Returns the real first-start receipt; the host scheduler owns subsequent segments.
pub(super) async fn start_configured_speech(
    host: Arc<dyn TtsPlaybackHost>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    interrupt: bool,
    wait_for_completion: bool,
    key: Option<(String, String)>,
    prepare: PrepareSpeech,
) -> Result<TtsPlaybackStatus, String> {
    let queue = queue_for(&host);
    let ticket = {
        let mut state = queue.0.lock().map_err(|error| error.to_string())?;
        if key.is_some() && state.pending.iter().any(|entry| entry.key == key) {
            return Err("Speech playback ID is already in use".into());
        }
        if interrupt {
            state.pending.clear();
            state.playing = None;
            host.stopSpeech().map_err(|error| error.to_string())?;
        }
        let ticket = state.next;
        state.next = state.next.wrapping_add(1);
        state.pending.push_back(SpeechEntry { ticket, key });
        ticket
    };
    let (sender, receiver) = oneshot::channel();
    let task_queue = queue.clone();
    let timer = scheduler.clone();
    let scheduled = scheduler.scheduleHostRuntimeBlockingAsyncTask(
        "configured-tts-playback",
        Box::new(move || {
            Box::pin(async move {
                let mut sender = Some(sender);
                let result = run_speech(
                    &task_queue, ticket, &host, &timer, prepare, &mut sender, wait_for_completion,
                ).await;
                task_queue.finish(ticket);
                if let Some(sender) = sender {
                    let _ = sender.send(result);
                } else if let Err(error) = result {
                    if error != CANCELLED { AppLogger::e("ConfiguredTtsPlayback", &error); }
                }
            })
        }),
    );
    if let Err(error) = scheduled {
        queue.finish(ticket);
        return Err(error.to_string());
    }
    receiver
        .await
        .map_err(|_| "TTS playback task ended before reporting its result".to_string())?
}

async fn run_speech(
    queue: &SpeechQueue,
    ticket: u64,
    host: &Arc<dyn TtsPlaybackHost>,
    timer: &Arc<dyn HostRuntimeTaskSchedulerHost>,
    prepare: PrepareSpeech,
    sender: &mut Option<oneshot::Sender<Result<TtsPlaybackStatus, String>>>,
    wait_for_completion: bool,
) -> Result<TtsPlaybackStatus, String> {
    // Non-interrupting requests wait for both earlier plugin requests and host playback.
    loop {
        if let Some(status) =
            queue.with_turn(ticket, |_| host.speechState().map_err(|e| e.to_string()))?
        {
            if !status.active {
                break;
            }
        }
        timer
            .waitForHostRuntimeDelay(120)
            .await
            .map_err(|e| e.to_string())?;
    }
    let starts = prepare()?;
    if starts.is_empty() {
        return Err("TTS provider returned no audio segments".into());
    }
    let mut started = None;
    for start in starts {
        // Recheck cancellation after synthesis, before any audio can start.
        let status = queue
            .with_turn(ticket, |state| {
                let status = start(host.as_ref())?;
                state.playing = Some((ticket, status.path.clone()));
                Ok(status)
            })?
            .ok_or("TTS playback lost its queue position")?;
        if !status.active {
            return Err(format!(
                "TTS host did not start playback: {}",
                status.details
            ));
        }
        if started.is_none() {
            if !wait_for_completion {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(Ok(status.clone()));
                }
            }
            started = Some(status);
        }
        let initial_path = queue.with_turn(ticket, |state| {
            let current = host.speechState().map_err(|error| error.to_string())?;
            state.playing = Some((ticket, current.path.clone()));
            Ok(current.path)
        })?.ok_or("TTS playback lost its queue position")?;
        loop {
            let status = queue
                .with_turn(ticket, |_| host.speechState().map_err(|e| e.to_string()))?
                .ok_or("TTS playback lost its queue position")?;
            if !status.active {
                break;
            }
            if status.path != initial_path {
                return Err("TTS playback was replaced by another host session".into());
            }
            timer
                .waitForHostRuntimeDelay(120)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    started.ok_or_else(|| "TTS provider returned no audio segments".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{HostResult, TtsPlaybackRequest};
    use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
    use std::sync::{mpsc, Condvar};
    use std::time::Duration;

    struct HostState {
        status: TtsPlaybackStatus,
        played: Vec<String>,
        refuse_start: bool,
    }

    struct TestHost {
        state: Mutex<HostState>,
        changed: Condvar,
    }

    impl TestHost {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                state: Mutex::new(HostState {
                    status: TtsPlaybackStatus {
                        path: String::new(),
                        active: false,
                        paused: false,
                        details: String::new(),
                    },
                    played: Vec::new(),
                    refuse_start: false,
                }),
                changed: Condvar::new(),
            })
        }

        fn complete(&self) {
            self.state.lock().unwrap().status.active = false;
        }

        fn wait_for_played(&self, count: usize) -> Vec<String> {
            let (state, timeout) = self
                .changed
                .wait_timeout_while(
                    self.state.lock().unwrap(),
                    Duration::from_secs(3),
                    |state| state.played.len() < count,
                )
                .unwrap();
            assert!(!timeout.timed_out(), "played: {:?}", state.played);
            state.played.clone()
        }
    }

    impl TtsPlaybackHost for TestHost {
        fn supportsSystemSpeech(&self) -> bool {
            true
        }
        fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
            let mut state = self.state.lock().unwrap();
            state.status.path = path.into();
            state.status.active = !state.refuse_start;
            state.status.paused = false;
            state.played.push(path.into());
            self.changed.notify_all();
            Ok(state.status.clone())
        }
        fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
            self.playAudio(&format!("system:{}", request.text))
        }
        fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
            self.state.lock().unwrap().status.paused = true;
            self.speechState()
        }
        fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
            self.state.lock().unwrap().status.paused = false;
            self.speechState()
        }
        fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
            self.complete();
            self.speechState()
        }
        fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
            Ok(self.state.lock().unwrap().status.clone())
        }
    }

    fn prepare(paths: &[&str]) -> Box<dyn FnOnce() -> Result<Vec<SpeechStart>, String> + Send> {
        let paths = paths.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Box::new(move || {
            Ok(paths
                .into_iter()
                .map(|path| {
                    Box::new(move |host: &dyn TtsPlaybackHost| {
                        host.playAudio(&path).map_err(|e| e.to_string())
                    }) as SpeechStart
                })
                .collect())
        })
    }

    fn enqueue(
        host: Arc<TestHost>,
        interrupt: bool,
        prepare: PrepareSpeech,
    ) -> mpsc::Receiver<Result<TtsPlaybackStatus, String>> {
        let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
        let (tx, rx) = mpsc::channel();
        let worker_scheduler = scheduler.clone();
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "test-speech-start",
                Box::new(move || {
                    Box::pin(async move {
                        let _ = tx.send(
                            start_configured_speech(host, worker_scheduler, interrupt, false, None, prepare)
                                .await,
                        );
                    })
                }),
            )
            .unwrap();
        rx
    }

    fn receipt(
        rx: mpsc::Receiver<Result<TtsPlaybackStatus, String>>,
    ) -> Result<TtsPlaybackStatus, String> {
        rx.recv_timeout(Duration::from_secs(3))
            .expect("speech start timed out")
    }

    #[test]
    fn generated_segments_wait_for_completion_and_preserve_order() {
        let host = TestHost::new();
        assert_eq!(
            receipt(enqueue(host.clone(), true, prepare(&["first", "second"])))
                .unwrap()
                .path,
            "first"
        );
        assert_eq!(host.state.lock().unwrap().played, ["first"]);
        host.complete();
        assert_eq!(host.wait_for_played(2), ["first", "second"]);
        host.complete();
    }

    #[test]
    fn paused_audio_does_not_advance_to_the_next_segment() {
        let host = TestHost::new();
        receipt(enqueue(host.clone(), true, prepare(&["first", "second"]))).unwrap();
        host.pauseSpeech().unwrap();
        let (state, timeout) = host
            .changed
            .wait_timeout_while(
                host.state.lock().unwrap(),
                Duration::from_millis(280),
                |s| s.played.len() == 1,
            )
            .unwrap();
        assert!(timeout.timed_out());
        assert_eq!(state.played, ["first"]);
        drop(state);
        host.resumeSpeech().unwrap();
        host.complete();
        host.wait_for_played(2);
        host.complete();
    }

    #[test]
    fn noninterrupting_speech_waits_for_existing_host_audio() {
        let host = TestHost::new();
        host.playAudio("external").unwrap();
        let rx = enqueue(host.clone(), false, prepare(&["queued"]));
        assert!(matches!(
            rx.recv_timeout(Duration::from_millis(160)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(host.speechState().unwrap().active);
        host.complete();
        assert_eq!(receipt(rx).unwrap().path, "queued");
        assert_eq!(host.wait_for_played(2), ["external", "queued"]);
        host.complete();
    }

    #[test]
    fn interrupt_discards_the_previous_utterance_remaining_segments() {
        let host = TestHost::new();
        receipt(enqueue(
            host.clone(),
            true,
            prepare(&["old-first", "old-second"]),
        ))
        .unwrap();
        receipt(enqueue(host.clone(), true, prepare(&["new"]))).unwrap();
        assert_eq!(host.state.lock().unwrap().played, ["old-first", "new"]);
        host.complete();
        // A final queued request acts as a barrier after both workers observe completion.
        receipt(enqueue(host.clone(), false, prepare(&["barrier"]))).unwrap();
        assert_eq!(host.wait_for_played(3), ["old-first", "new", "barrier"]);
        host.complete();
    }

    #[test]
    fn stop_during_synthesis_prevents_late_audio_from_starting() {
        let host = TestHost::new();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let rx = enqueue(
            host.clone(),
            true,
            Box::new(move || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                prepare(&["stale"])()
            }),
        );
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let playback_host: Arc<dyn TtsPlaybackHost> = host.clone();
        replace_playback(&playback_host, || {
            host.stopSpeech().map_err(|e| e.to_string())
        })
        .unwrap();
        release_tx.send(()).unwrap();
        assert!(receipt(rx).unwrap_err().contains("cancelled"));
        assert!(host.state.lock().unwrap().played.is_empty());
    }

    #[test]
    fn synthesis_and_empty_audio_failures_are_returned_before_success() {
        for error in [
            "provider unavailable",
            "TTS provider returned no audio segments",
        ] {
            let host = TestHost::new();
            let prepare = if error == "provider unavailable" {
                Box::new(|| Err("provider unavailable".into()))
                    as Box<dyn FnOnce() -> Result<Vec<SpeechStart>, String> + Send>
            } else {
                prepare(&[])
            };
            assert_eq!(
                receipt(enqueue(host.clone(), true, prepare)).unwrap_err(),
                error
            );
            assert!(host.state.lock().unwrap().played.is_empty());
        }
    }

    #[test]
    fn host_refusing_to_start_is_reported_as_a_failure() {
        let host = TestHost::new();
        host.state.lock().unwrap().refuse_start = true;
        assert!(receipt(enqueue(host, true, prepare(&["audio"])))
            .unwrap_err()
            .contains("did not start"));
    }
}
