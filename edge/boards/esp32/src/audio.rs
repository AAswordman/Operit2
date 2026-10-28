//! Board-specific audio orchestration. The Core speech processor remains unchanged.
#![allow(non_snake_case)]
use crate::device_speech::DeviceSpeech;
use crate::wav::WavDecoder;
use operit_board_esp32::audio::{
    native::{Microphone, Speaker},
    MicrophonePcm,
};
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::CoreValue;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{Duration, Instant};

#[derive(Default)]
struct State {
    phase: &'static str,
    samples: u32,
    nonzero: u32,
    peak: u32,
    rightPeak: u32,
    clipped: u32,
    sum: i64,
    squares: u64,
    error: Option<String>,
    finished: bool,
}
struct Driver {
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    speech: Arc<DeviceSpeech>,
    busy: AtomicBool,
    stop: AtomicBool,
    state: Mutex<State>,
}
static DRIVER: OnceLock<Arc<Driver>> = OnceLock::new();
pub fn install(scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>, speech: Arc<DeviceSpeech>) {
    let _ = DRIVER.set(Arc::new(Driver {
        scheduler,
        speech,
        busy: AtomicBool::new(false),
        stop: AtomicBool::new(false),
        state: Mutex::new(State::default()),
    }));
}
fn driver() -> Result<Arc<Driver>, String> {
    DRIVER
        .get()
        .cloned()
        .ok_or_else(|| "Audio unavailable".into())
}
fn update(d: &Driver, phase: &'static str) {
    d.state.lock().unwrap().phase = phase;
}
fn stats(state: &mut State, pcm: i16) {
    state.samples += 1;
    state.clipped += u32::from(pcm == i16::MIN || pcm == i16::MAX);
    state.sum += pcm as i64;
    state.squares += (pcm as i64 * pcm as i64) as u64;
}
fn complete(d: &Driver, result: Result<(), String>) {
    if let Err(error) = &result {
        d.speech.cancel(error.clone());
    }
    let mut state = d.state.lock().unwrap();
    state.error = result.err();
    state.phase = "idle";
    state.finished = true;
    d.busy.store(false, Ordering::Release);
}
pub fn snapshot() -> serde_json::Value {
    let Ok(d) = driver() else {
        return serde_json::json!({"available":false});
    };
    let s = d.state.lock().unwrap();
    serde_json::json!({"available":true,"busy":d.busy.load(Ordering::Acquire),
        "phase":s.phase,"samples":s.samples,"nonzeroSamples":s.nonzero,"rawPeak24":s.peak,"rightRawPeak24":s.rightPeak,
        "clippedSamples":s.clipped,"sumPcm":s.sum,"sumSquaresPcm":s.squares,"error":s.error,
        "wiring":"INMP441 BCLK27/WS22/SD35 left; DAC26"})
}
pub fn takeResult() -> Option<Result<(), String>> {
    let d = driver().ok()?;
    let mut s = d.state.lock().unwrap();
    if !s.finished {
        return None;
    }
    s.finished = false;
    Some(s.error.clone().map_or(Ok(()), Err))
}
fn launch(test: bool, first: Option<CoreValue>) -> Result<(), String> {
    let d = driver()?;
    d.busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "Audio is busy")?;
    d.stop.store(false, Ordering::Release);
    *d.state.lock().unwrap() = State {
        phase: if test {
            "self-test"
        } else if first.is_some() {
            "playback"
        } else {
            "recording"
        },
        ..State::default()
    };
    let worker = d.clone();
    let result = d.scheduler.scheduleHostRuntimeBlockingAsyncTask(
        "board-audio",
        Box::new(move || {
            Box::pin(async move {
                // Background Space/NVS materialization can pause this worker
                // longer than the fixed DMA ring. Reuse device work admission.
                let _work = crate::space_work::acquire().await;
                let result = if let Some(packet) = first {
                    playback(&worker, packet).await
                } else if test {
                    self_test(&worker).await
                } else {
                    record(&worker).await
                };
                complete(&worker, result);
            })
        }),
    );
    if let Err(error) = result {
        d.busy.store(false, Ordering::Release);
        return Err(error.to_string());
    }
    Ok(())
}
pub fn test() -> Result<(), String> {
    launch(true, None)
}
pub fn toggleRecording() -> Result<(), String> {
    let d = driver()?;
    if d.busy.load(Ordering::Acquire) {
        if d.state.lock().unwrap().phase == "recording" {
            d.stop.store(true, Ordering::Release);
            return Ok(());
        }
        return Err("Audio is busy".into());
    }
    d.speech.beginRecognition()?;
    if let Err(error) = launch(false, None) {
        d.speech.cancel(error.clone());
        return Err(error);
    }
    Ok(())
}
pub fn debug(argument: &str) -> Result<serde_json::Value, String> {
    match argument {
        "" | "status" => {}
        "test" => test()?,
        "record" => toggleRecording()?,
        "stop" => {
            let d = driver()?;
            d.stop.store(true, Ordering::Release);
        }
        text if text.starts_with("speak ") => {
            driver()?.speech.synthesize(text[6..].to_owned())?;
        }
        _ => return Err("Use status, test, record, stop, or speak <text>".into()),
    }
    Ok(snapshot())
}
pub fn pump() {
    let Ok(d) = driver() else {
        return;
    };
    if !d.busy.load(Ordering::Acquire) {
        if let Some(packet) = d.speech.takePlayback() {
            if let Err(error) = launch(false, Some(packet)) {
                d.speech.cancel(error);
            }
        }
    }
}
async fn self_test(d: &Driver) -> Result<(), String> {
    update(d, "speaker-test");
    {
        let mut speaker = Speaker::open(16000)?;
        let mut wave = [128u8; 256];
        // 500 Hz, 250 ms, +/-8 DAC steps: deliberately quiet initial test.
        for block in 0..16 {
            for (i, s) in wave.iter_mut().enumerate() {
                let n = block * 256 + i;
                *s = if n < 4000 {
                    if n % 32 < 16 {
                        136
                    } else {
                        120
                    }
                } else {
                    128
                };
            }
            speaker.write(&wave)?;
            tokio::task::yield_now().await;
        }
        speaker.finish()?;
    }
    update(d, "microphone-test");
    capture(d, true).await
}
async fn capture(d: &Driver, test: bool) -> Result<(), String> {
    let id = if test {
        String::new()
    } else {
        d.speech.recordingId().ok_or("Speech capture ended")?
    };
    let mut microphone = Microphone::open()?;
    let mut filter = MicrophonePcm::default();
    let deadline = Instant::now() + Duration::from_secs(if test { 3 } else { 300 });
    let mut discard = 8; // INMP441 startup settling: 2048 samples at 16 kHz.
    while Instant::now() < deadline && !d.stop.load(Ordering::Acquire) {
        if !test && !d.speech.isRecording(&id) {
            return Err("Speech capture cancelled or disconnected".into());
        }
        let block = microphone.read()?;
        if discard > 0 {
            discard -= 1;
            continue;
        }
        let mut pcm = Vec::new();
        if !test {
            pcm.try_reserve_exact(block.samples.len() * 2)
                .map_err(|_| "Microphone PCM allocation failed")?;
        }
        {
            // Publish one DMA block at a time: locking for every 16 kHz sample
            // can let diagnostic/UI contention overrun the bounded RX ring.
            let mut state = d.state.lock().unwrap();
            state.nonzero += block.nonzero;
            state.peak = state.peak.max(block.peak);
            state.rightPeak = state.rightPeak.max(block.rightPeak);
            for raw in block.samples {
                let sample = filter.sample16(raw);
                stats(&mut state, sample);
                if !test {
                    pcm.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
        if !test {
            tokio::time::timeout(Duration::from_secs(1), d.speech.appendPcmFor(&id, pcm))
                .await
                .map_err(|_| "Microphone transport stalled")??;
        }
        tokio::task::yield_now().await;
    }
    if test {
        if d.stop.load(Ordering::Acquire) {
            return Err("Audio test stopped".into());
        }
        if d.state.lock().unwrap().nonzero == 0 {
            return Err(
                "Microphone returned only zero samples; check power, wiring and L/R".into(),
            );
        }
        Ok(())
    } else {
        tokio::time::timeout(Duration::from_secs(1), d.speech.finishRecognitionFor(&id))
            .await
            .map_err(|_| "Speech completion stalled")?
    }
}
async fn record(d: &Driver) -> Result<(), String> {
    update(d, "recording");
    capture(d, false).await
}

async fn playback(d: &Driver, first: CoreValue) -> Result<(), String> {
    let mut decoder = None;
    let mut speaker: Option<Speaker> = None;
    let mut rate = 0;
    let mut request = None;
    let mut next = Some(first);
    let mut last = Instant::now();
    loop {
        if d.stop.load(Ordering::Acquire) {
            return Err("Audio playback stopped".into());
        }
        let packet = match next.take().or_else(|| d.speech.takePlayback()) {
            Some(packet) => packet,
            None => {
                if Instant::now().duration_since(last) > Duration::from_secs(15) {
                    return Err("Audio playback timed out".into());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
                continue;
            }
        };
        last = Instant::now();
        let CoreValue::Map(fields) = packet else {
            return Err("Invalid playback packet".into());
        };
        let Some(CoreValue::String(kind)) = fields.get("type") else {
            return Err("Missing playback kind".into());
        };
        let Some(CoreValue::String(id)) = fields.get("id") else {
            return Err("Missing playback identity".into());
        };
        if request.as_ref().is_some_and(|previous| previous != id) {
            return Err("Playback request changed".into());
        }
        if request.is_none() {
            request = Some(id.clone());
        }
        // A cancellation flushes DeviceSpeech's queue. Stop any already-held block too.
        if !d.speech.canPlay(id) {
            return Err("Playback cancelled".into());
        }
        match kind.as_str() {
            "tts.segment" => {
                if let Some(previous) = decoder.take() {
                    let previous: WavDecoder = previous;
                    previous.finish()?;
                }
                if let Some(s) = speaker.as_mut() {
                    s.finish()?;
                }
                speaker = None;
                let mime = match fields.get("contentType") {
                    Some(CoreValue::String(s)) => s.as_str(),
                    _ => "audio/wav",
                };
                if !matches!(mime, "audio/wav" | "audio/x-wav" | "audio/wave") {
                    return Err("Hardware speaker requires PCM16 WAV from Core".into());
                }
                decoder = Some(WavDecoder::default());
            }
            "tts.data" => {
                let Some(CoreValue::Bytes(bytes)) = fields.get("audio") else {
                    return Err("Missing playback samples".into());
                };
                decoder.as_mut().ok_or("Missing audio segment")?.push(
                    bytes,
                    |newRate, samples| {
                        if speaker.is_none() {
                            speaker = Some(Speaker::open(newRate)?);
                            rate = newRate;
                        }
                        if newRate != rate {
                            return Err("WAV sample rate changed".into());
                        }
                        speaker.as_mut().unwrap().write_pcm16(samples)
                    },
                )?;
            }
            "tts.end" => {
                decoder.as_ref().ok_or("Empty playback")?.finish()?;
                if let Some(s) = speaker.as_mut() {
                    s.finish()?;
                }
                return Ok(());
            }
            _ => return Err("Unexpected playback packet".into()),
        }
        tokio::task::yield_now().await;
    }
}
