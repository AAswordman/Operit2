use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::LocalEngineManifest::LocalPlatform;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelKind {
    SpeechToText,
    TextToSpeech,
    Chat,
    Embedding,
}

impl LocalModelKind {
    /// Returns the stable storage segment for this local model kind.
    pub fn storageSegment(&self) -> &'static str {
        match self {
            Self::SpeechToText => "stt",
            Self::TextToSpeech => "tts",
            Self::Chat => "chat",
            Self::Embedding => "embedding",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalEngineKind {
    SherpaOnnx,
    SherpaNcnn,
    Piper,
    LlamaCpp,
    Mnn,
    OnnxRuntime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineRequirement {
    pub engineId: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelDriver {
    SherpaOnnxStreamingTransducer {
        encoder: String,
        decoder: String,
        joiner: String,
        tokens: String,
        modelType: String,
    },
    SherpaOnnxVits {
        model: String,
        lexicon: String,
        tokens: String,
        ruleFsts: Vec<String>,
        ruleFars: Vec<String>,
        speakerCount: i32,
    },
    SherpaOnnxMatcha {
        acousticModel: String,
        vocoder: String,
        lexicon: String,
        tokens: String,
        ruleFsts: Vec<String>,
        ruleFars: Vec<String>,
        speakerCount: i32,
    },
    SherpaOnnxKitten {
        model: String,
        voices: String,
        tokens: String,
        dataDir: String,
        speakerCount: i32,
    },
    SherpaOnnxWebAsrBundle {
        recognizerScript: String,
        runtimeScript: String,
        runtimeWasm: String,
        runtimeData: String,
    },
    SherpaOnnxWebTtsBundle {
        ttsScript: String,
        runtimeScript: String,
        runtimeWasm: String,
        runtimeData: String,
        speakerCount: i32,
    },
    SherpaNcnnStreamingTransducer {
        encoderParam: String,
        encoderBin: String,
        decoderParam: String,
        decoderBin: String,
        joinerParam: String,
        joinerBin: String,
        tokens: String,
    },
}

impl LocalModelDriver {
    /// Returns whether this driver has an inference implementation on one platform.
    pub fn supportsPlatform(&self, platform: &LocalPlatform) -> bool {
        match self {
            Self::SherpaOnnxStreamingTransducer { .. }
            | Self::SherpaOnnxVits { .. }
            | Self::SherpaOnnxMatcha { .. }
            | Self::SherpaOnnxKitten { .. } => matches!(
                platform,
                LocalPlatform::Windows
                    | LocalPlatform::Linux
                    | LocalPlatform::Macos
                    | LocalPlatform::Android
                    | LocalPlatform::Ohos
                    | LocalPlatform::Ios
            ),
            Self::SherpaOnnxWebAsrBundle { .. } | Self::SherpaOnnxWebTtsBundle { .. } => {
                *platform == LocalPlatform::Web
            }
            Self::SherpaNcnnStreamingTransducer { .. } => false,
        }
    }
}

impl LocalEngineKind {
    /// Returns the stable storage segment for this local engine kind.
    pub fn storageSegment(&self) -> &'static str {
        match self {
            Self::SherpaOnnx => "sherpa_onnx",
            Self::SherpaNcnn => "sherpa_ncnn",
            Self::Piper => "piper",
            Self::LlamaCpp => "llama_cpp",
            Self::Mnn => "mnn",
            Self::OnnxRuntime => "onnxruntime",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelSourceKind {
    HuggingFace,
    ModelScope,
    DirectHttp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelSource {
    pub id: String,
    pub kind: LocalModelSourceKind,
    pub repository: String,
    pub revision: String,
    pub baseUrl: String,
}

impl LocalModelSource {
    /// Builds a concrete download URL for one relative model file path.
    pub fn fileUrl(&self, relativePath: &str) -> String {
        let base = self.baseUrl.trim_end_matches('/');
        let path = relativePath.trim_start_matches('/');
        format!("{base}/{path}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelFile {
    pub relativePath: String,
    pub sha256: String,
    pub byteSize: u64,
    pub sourceId: String,
}

impl LocalModelFile {
    /// Verifies that the supplied bytes match the manifest checksum.
    pub fn verifySha256(&self, bytes: &[u8]) -> bool {
        let digest = Sha256::digest(bytes);
        let calculated = format!("{digest:x}");
        calculated.eq_ignore_ascii_case(self.sha256.trim())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelArchiveFormat {
    TarBz2,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelArchive {
    pub archiveId: String,
    pub relativePath: String,
    pub sha256: String,
    pub byteSize: u64,
    pub sourceId: String,
    pub archiveFormat: LocalModelArchiveFormat,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelInstallSource {
    Files,
    Archives { archives: Vec<LocalModelArchive> },
}

impl Default for LocalModelInstallSource {
    /// Returns the manifest install source used by direct file lists.
    fn default() -> Self {
        Self::Files
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelManifest {
    pub id: String,
    pub version: String,
    pub displayName: String,
    pub description: String,
    pub kind: LocalModelKind,
    pub engine: LocalEngineKind,
    pub license: String,
    pub homepage: String,
    pub languages: Vec<String>,
    pub tags: Vec<String>,
    pub engineRequirement: Option<LocalEngineRequirement>,
    pub driver: Option<LocalModelDriver>,
    pub sources: Vec<LocalModelSource>,
    #[serde(default)]
    pub installSource: LocalModelInstallSource,
    pub files: Vec<LocalModelFile>,
}

impl LocalModelManifest {
    /// Returns whether this model declares a driver implemented on one platform.
    pub fn supportsPlatform(&self, platform: &LocalPlatform) -> bool {
        self.driver
            .as_ref()
            .map(|driver| driver.supportsPlatform(platform))
            .unwrap_or(false)
    }

    /// Returns the total byte size declared by the manifest download source.
    pub fn declaredByteSize(&self) -> u64 {
        match &self.installSource {
            LocalModelInstallSource::Files => self.files.iter().map(|file| file.byteSize).sum(),
            LocalModelInstallSource::Archives { archives } => {
                archives.iter().map(|archive| archive.byteSize).sum()
            }
        }
    }

    /// Returns the stable registry key for this manifest version.
    pub fn registryKey(&self) -> String {
        format!("{}@{}", self.id.trim(), self.version.trim())
    }

    /// Returns the download source matching the supplied source id.
    pub fn sourceById(&self, sourceId: &str) -> Option<&LocalModelSource> {
        let sourceId = sourceId.trim();
        self.sources.iter().find(|source| source.id == sourceId)
    }

    /// Returns the download source for one manifest file.
    pub fn sourceForFile(&self, file: &LocalModelFile) -> Option<&LocalModelSource> {
        self.sourceById(&file.sourceId)
    }

    /// Returns the download source for one manifest archive.
    pub fn sourceForArchive(&self, archive: &LocalModelArchive) -> Option<&LocalModelSource> {
        self.sourceById(&archive.sourceId)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies checksum validation for a manifest file record.
    #[test]
    fn localModelFileVerifiesSha256() {
        let file = LocalModelFile {
            relativePath: "tokens.txt".to_string(),
            sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".to_string(),
            byteSize: 5,
            sourceId: "main".to_string(),
        };

        assert!(file.verifySha256(b"hello"));
        assert!(!file.verifySha256(b"HELLO"));
    }

    /// Verifies source URL construction trims only boundary slashes.
    #[test]
    fn localModelSourceBuildsFileUrl() {
        let source = LocalModelSource {
            id: "main".to_string(),
            kind: LocalModelSourceKind::HuggingFace,
            repository: "owner/repo".to_string(),
            revision: "rev".to_string(),
            baseUrl: "https://example.test/models/".to_string(),
        };

        assert_eq!(
            source.fileUrl("/dir/model.bin"),
            "https://example.test/models/dir/model.bin"
        );
    }
}
