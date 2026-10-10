//! Audio uses ordinary Link Call controls and binary Link Push items.
use operit_host_api::AudioInputFormat;
use serde::{Deserialize, Serialize};
pub const EDGE_AUDIO_TARGET: &str = "edge.audio";
pub const CORE_AUDIO_INGRESS_TARGET: &str = "edge.audio.ingress";
pub const CORE_AUDIO_INGRESS_METHOD: &str = "upload";
pub const AUDIO_MAX_DURATION_MS: u32 = 300_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartAudioInput {
    pub streamId: String,
    pub receiverNodeId: String,
    pub inputId: String,
    pub format: AudioInputFormat,
    pub maxDurationMs: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioIngressOpen {
    pub streamId: String,
    pub format: AudioInputFormat,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopAudioInput {
    pub streamId: String,
}
