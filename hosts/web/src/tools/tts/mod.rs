use js_sys::Object;
use operit_host_api::{HostResult, TtsPlaybackHost, TtsPlaybackRequest, TtsPlaybackStatus};
use wasm_bindgen::prelude::*;

use crate::common::{call_tts_playback, read_bool_property, read_string_property, set_property};

#[derive(Clone, Debug, Default)]
pub struct WebTtsPlaybackHost;

unsafe impl Send for WebTtsPlaybackHost {}
unsafe impl Sync for WebTtsPlaybackHost {}

impl WebTtsPlaybackHost {
    /// Creates a browser TTS playback host.
    pub fn new() -> Self {
        Self
    }
}

impl TtsPlaybackHost for WebTtsPlaybackHost {
    /// Reports browser speechSynthesis availability through the web host.
    fn supportsSystemSpeech(&self) -> bool {
        true
    }

    /// Starts one generated speech audio file in the browser host.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback("playAudio", &[JsValue::from_str(path)])?)
    }

    /// Starts one browser system speech request.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback(
            "speakText",
            &[tts_request_to_js(request)],
        )?)
    }

    /// Pauses the active browser speech session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback("pauseSpeech", &[])?)
    }

    /// Resumes the active browser speech session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback("resumeSpeech", &[])?)
    }

    /// Stops the active browser speech session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback("stopSpeech", &[])?)
    }

    /// Returns the current browser speech session state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus> {
        tts_status(call_tts_playback("speechState", &[])?)
    }
}

/// Converts one TTS request into its browser bridge representation.
fn tts_request_to_js(request: TtsPlaybackRequest) -> JsValue {
    let object = Object::new();
    set_property(&object, "text", JsValue::from_str(&request.text));
    set_property(&object, "voice", JsValue::from_str(&request.voice));
    set_property(&object, "locale", JsValue::from_str(&request.locale));
    set_property(&object, "speed", JsValue::from_f64(request.speed));
    set_property(&object, "pitch", JsValue::from_f64(request.pitch));
    set_property(&object, "interrupt", JsValue::from_bool(request.interrupt));
    object.into()
}

/// Parses one browser TTS status response.
fn tts_status(value: JsValue) -> HostResult<TtsPlaybackStatus> {
    Ok(TtsPlaybackStatus {
        path: read_string_property(&value, "path")?,
        active: read_bool_property(&value, "active")?,
        paused: read_bool_property(&value, "paused")?,
        details: read_string_property(&value, "details")?,
    })
}
