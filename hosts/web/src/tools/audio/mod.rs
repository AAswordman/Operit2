use operit_host_api::{
    AudioPlaybackHost, AudioPlaybackStatus, HostResult, MusicPlaybackRequest, MusicPlaybackStatus,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    call_music_playback, music_playback_request_to_js, music_playback_status, read_bool_property,
    read_string_property,
};

#[derive(Clone, Debug, Default)]
pub struct WebAudioPlaybackHost;

unsafe impl Send for WebAudioPlaybackHost {}
unsafe impl Sync for WebAudioPlaybackHost {}

impl WebAudioPlaybackHost {
    pub fn new() -> Self {
        Self
    }
}

impl AudioPlaybackHost for WebAudioPlaybackHost {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus> {
        let value = call_music_playback("playAudio", &[JsValue::from_str(path)])?;
        Ok(AudioPlaybackStatus {
            path: read_string_property(&value, "path")?,
            started: read_bool_property(&value, "started")?,
            details: read_string_property(&value, "details")?,
        })
    }

    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback(
            "playMusic",
            &[music_playback_request_to_js(request)],
        )?)
    }

    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback("pauseMusic", &[])?)
    }

    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback("resumeMusic", &[])?)
    }

    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback("stopMusic", &[])?)
    }

    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback(
            "seekMusic",
            &[JsValue::from_f64(positionMs as f64)],
        )?)
    }

    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback(
            "setMusicVolume",
            &[JsValue::from_f64(volume)],
        )?)
    }

    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus> {
        music_playback_status(call_music_playback("musicStatus", &[])?)
    }
}
