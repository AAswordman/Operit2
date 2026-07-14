use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalPlatform {
    Android,
    Ohos,
    Ios,
    Web,
    Windows,
    Linux,
    Macos,
}

impl LocalPlatform {
    /// Returns the stable storage segment for this platform.
    pub fn storageSegment(&self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Ohos => "ohos",
            Self::Ios => "ios",
            Self::Web => "web",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalArchitecture {
    Aarch64,
    X86_64,
    Armv7,
    X86,
    Wasm32,
}

impl LocalArchitecture {
    /// Returns the stable storage segment for this architecture.
    pub fn storageSegment(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_64 => "x86_64",
            Self::Armv7 => "armv7",
            Self::X86 => "x86",
            Self::Wasm32 => "wasm32",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalPlatformTarget {
    pub platform: LocalPlatform,
    pub architecture: LocalArchitecture,
}

impl LocalPlatformTarget {
    /// Returns the stable storage segment for this platform target.
    pub fn storageSegment(&self) -> String {
        format!(
            "{}-{}",
            self.platform.storageSegment(),
            self.architecture.storageSegment()
        )
    }

    /// Resolves the platform target compiled into the current process.
    pub fn current() -> Result<Self, String> {
        let platform = if cfg!(target_env = "ohos") {
            LocalPlatform::Ohos
        } else if cfg!(target_arch = "wasm32") {
            LocalPlatform::Web
        } else if cfg!(target_os = "android") {
            LocalPlatform::Android
        } else if cfg!(target_os = "ios") {
            LocalPlatform::Ios
        } else if cfg!(target_os = "windows") {
            LocalPlatform::Windows
        } else if cfg!(all(target_os = "linux", not(target_env = "ohos"))) {
            LocalPlatform::Linux
        } else if cfg!(target_os = "macos") {
            LocalPlatform::Macos
        } else {
            return Err("local model engines are unsupported on this platform".to_string());
        };
        let architecture = if cfg!(target_arch = "aarch64") {
            LocalArchitecture::Aarch64
        } else if cfg!(target_arch = "x86_64") {
            LocalArchitecture::X86_64
        } else if cfg!(target_arch = "arm") {
            LocalArchitecture::Armv7
        } else if cfg!(target_arch = "x86") {
            LocalArchitecture::X86
        } else if cfg!(target_arch = "wasm32") {
            LocalArchitecture::Wasm32
        } else {
            return Err("local model engines are unsupported on this architecture".to_string());
        };
        Ok(Self {
            platform,
            architecture,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalEngineArchiveFormat {
    TarBz2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalEngineDelivery {
    #[default]
    DownloadArchive,
    Embedded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineArtifact {
    pub target: LocalPlatformTarget,
    #[serde(default)]
    pub delivery: LocalEngineDelivery,
    pub url: String,
    pub sha256: String,
    pub byteSize: u64,
    pub archiveFormat: LocalEngineArchiveFormat,
    pub archiveRoot: String,
    pub sttExecutable: Option<String>,
    pub ttsExecutable: Option<String>,
    pub androidLibraryDir: Option<String>,
    #[serde(default)]
    pub ohosLibraryDir: Option<String>,
    #[serde(default)]
    pub iosFrameworkDir: Option<String>,
    #[serde(default)]
    pub webRuntimeDir: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalEngineManifest {
    pub id: String,
    pub version: String,
    pub displayName: String,
    pub description: String,
    pub license: String,
    pub homepage: String,
    pub artifacts: Vec<LocalEngineArtifact>,
}

impl LocalEngineManifest {
    /// Returns the stable registry key for this engine version.
    pub fn registryKey(&self) -> String {
        format!("{}@{}", self.id.trim(), self.version.trim())
    }

    /// Returns the artifact matching an exact platform target.
    pub fn artifactForTarget(&self, target: &LocalPlatformTarget) -> Option<&LocalEngineArtifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.target == *target)
    }
}
