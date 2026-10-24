//! Registers process-local media and returns the link used by chat providers.
use super::results::MediaRegistrationResultData;
use super::JsFuture;
use serde::{Deserialize, Serialize};

/// Selects the pool and link type for one registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "video")]
    Video,
}

/// Registers media that the caller already holds as base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaRegisterOptions {
    /// Selects image, audio, or video.
    pub r#type: MediaKind,
    /// Contains the raw or data-URL base64 payload.
    pub base64: String,
    /// Supplies a MIME type when the payload does not identify itself.
    pub mimeType: Option<String>,
}

/// Registers media stored at a virtual file-system path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaRegisterFromPathOptions {
    /// Selects image, audio, or video.
    pub r#type: MediaKind,
    /// Identifies the virtual file-system path to read.
    pub path: String,
}

/// Registers image, audio, and video bytes in the host media pools.
#[allow(non_snake_case)]
pub trait MediaHost: Send + Sync {
    /// Registers base64 media and returns its canonical link.
    fn register(&self, options: MediaRegisterOptions) -> JsFuture<MediaRegistrationResultData>;
    /// Reads a virtual file and registers it as media.
    fn registerFromPath(
        &self,
        options: MediaRegisterFromPathOptions,
    ) -> JsFuture<MediaRegistrationResultData>;
}
