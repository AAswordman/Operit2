use crate::LocalEngineManifest::{
    LocalArchitecture, LocalEngineArchiveFormat, LocalEngineArtifact, LocalEngineDelivery,
    LocalEngineManifest, LocalPlatform, LocalPlatformTarget,
};

const SHERPA_ONNX_ENGINE_ID: &str = "sherpa-onnx";
const SHERPA_ONNX_ENGINE_VERSION: &str = "1.13.2";
const SHERPA_ONNX_WEB_ASSET_BASE_URL: &str = "https://models.operit.app/sherpa-onnx/v1.13.2";

pub struct LocalEngineCatalog;

impl LocalEngineCatalog {
    /// Returns every built-in local inference engine package.
    pub fn manifests() -> Vec<LocalEngineManifest> {
        vec![Self::sherpaOnnx()]
    }

    /// Returns the Sherpa ONNX engine package for desktop, Android, and OHOS targets.
    pub fn sherpaOnnx() -> LocalEngineManifest {
        LocalEngineManifest {
            id: SHERPA_ONNX_ENGINE_ID.to_string(),
            version: SHERPA_ONNX_ENGINE_VERSION.to_string(),
            displayName: "Sherpa ONNX".to_string(),
            description: "Local STT and TTS inference engine distributed by k2-fsa.".to_string(),
            license: "apache-2.0".to_string(),
            homepage: "https://github.com/k2-fsa/sherpa-onnx".to_string(),
            artifacts: vec![
                desktopArtifact(
                    LocalPlatform::Windows,
                    LocalArchitecture::X86_64,
                    "sherpa-onnx-v1.13.2-win-x64-shared-MD-Release.tar.bz2",
                    "f91f488186e797dd9e9bc2a3dcbe18ddd244627af5d9fa3707f7a2f3bc4032ce",
                    19_164_500,
                    "sherpa-onnx-v1.13.2-win-x64-shared-MD-Release",
                    "bin/sherpa-onnx.exe",
                    "bin/sherpa-onnx-offline-tts.exe",
                ),
                desktopArtifact(
                    LocalPlatform::Linux,
                    LocalArchitecture::X86_64,
                    "sherpa-onnx-v1.13.2-linux-x64-shared.tar.bz2",
                    "1ef6741535f7af4d69e394fd440a807108036d26ed4f542660191019da5c0daa",
                    26_825_365,
                    "sherpa-onnx-v1.13.2-linux-x64-shared",
                    "bin/sherpa-onnx",
                    "bin/sherpa-onnx-offline-tts",
                ),
                desktopArtifact(
                    LocalPlatform::Linux,
                    LocalArchitecture::Aarch64,
                    "sherpa-onnx-v1.13.2-linux-aarch64-shared-cpu.tar.bz2",
                    "b54178420e9e6ff6c7f308b5f1cde827215b38393356ee0bd2b7595c648b330b",
                    26_674_802,
                    "sherpa-onnx-v1.13.2-linux-aarch64-shared-cpu",
                    "bin/sherpa-onnx",
                    "bin/sherpa-onnx-offline-tts",
                ),
                desktopArtifact(
                    LocalPlatform::Macos,
                    LocalArchitecture::X86_64,
                    "sherpa-onnx-v1.13.2-osx-x64-shared.tar.bz2",
                    "5accc61eca4a69fc8860f2078c55f61ca96f67b4c311badffa0d6924ca6e1911",
                    28_956_076,
                    "sherpa-onnx-v1.13.2-osx-x64-shared",
                    "bin/sherpa-onnx",
                    "bin/sherpa-onnx-offline-tts",
                ),
                desktopArtifact(
                    LocalPlatform::Macos,
                    LocalArchitecture::Aarch64,
                    "sherpa-onnx-v1.13.2-osx-arm64-shared.tar.bz2",
                    "50c5c04d93113602432a13454d6bf8e5d2624206b985fbd0dd4698454ae6c509",
                    25_914_829,
                    "sherpa-onnx-v1.13.2-osx-arm64-shared",
                    "bin/sherpa-onnx",
                    "bin/sherpa-onnx-offline-tts",
                ),
                iosArtifact(LocalArchitecture::Aarch64),
                iosArtifact(LocalArchitecture::X86_64),
                webArtifact(),
                androidArtifact(LocalArchitecture::Aarch64, "arm64-v8a"),
                androidArtifact(LocalArchitecture::X86_64, "x86_64"),
                androidArtifact(LocalArchitecture::Armv7, "armeabi-v7a"),
                androidArtifact(LocalArchitecture::X86, "x86"),
                ohosArtifact(
                    LocalArchitecture::Aarch64,
                    "arm64-v8a",
                    "21ab8ad0b8918f2e05ed691b35cb573e87771adcb0db9c194f26d40be37df247",
                    5_600_834,
                ),
                ohosArtifact(
                    LocalArchitecture::Armv7,
                    "armeabi-v7a",
                    "b22f49b44b7173b24be7b906ea09a0c0813c0ee8e1cdd607bced2b1fdc449246",
                    6_498_309,
                ),
                ohosArtifact(
                    LocalArchitecture::X86_64,
                    "x86_64",
                    "08912ff3de6b6d37b6c82793c5bf69c09eaa1b8c78674ecfbb968debbfdb0353",
                    6_293_399,
                ),
            ],
        }
    }
}

/// Builds one desktop Sherpa ONNX archive artifact.
fn desktopArtifact(
    platform: LocalPlatform,
    architecture: LocalArchitecture,
    archiveName: &str,
    sha256: &str,
    byteSize: u64,
    archiveRoot: &str,
    sttExecutable: &str,
    ttsExecutable: &str,
) -> LocalEngineArtifact {
    LocalEngineArtifact {
        target: LocalPlatformTarget {
            platform,
            architecture,
        },
        delivery: LocalEngineDelivery::DownloadArchive,
        url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{SHERPA_ONNX_ENGINE_VERSION}/{archiveName}"
        ),
        sha256: sha256.to_string(),
        byteSize,
        archiveFormat: LocalEngineArchiveFormat::TarBz2,
        archiveRoot: archiveRoot.to_string(),
        sttExecutable: Some(sttExecutable.to_string()),
        ttsExecutable: Some(ttsExecutable.to_string()),
        androidLibraryDir: None,
        ohosLibraryDir: None,
        iosFrameworkDir: None,
        webRuntimeDir: None,
    }
}

/// Builds one iOS Sherpa ONNX xcframework archive artifact.
fn iosArtifact(architecture: LocalArchitecture) -> LocalEngineArtifact {
    LocalEngineArtifact {
        target: LocalPlatformTarget {
            platform: LocalPlatform::Ios,
            architecture,
        },
        delivery: LocalEngineDelivery::Embedded,
        url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{SHERPA_ONNX_ENGINE_VERSION}/sherpa-onnx-v{SHERPA_ONNX_ENGINE_VERSION}-ios.tar.bz2"
        ),
        sha256: "2886a04df4f8d5066c6c8b6e712278d65d7b60fc9e45990223df50262861d38b"
            .to_string(),
        byteSize: 77_611_169,
        archiveFormat: LocalEngineArchiveFormat::TarBz2,
        archiveRoot: "build-ios".to_string(),
        sttExecutable: None,
        ttsExecutable: None,
        androidLibraryDir: None,
        ohosLibraryDir: None,
        iosFrameworkDir: Some("sherpa-onnx.xcframework".to_string()),
        webRuntimeDir: None,
    }
}

/// Builds one browser Sherpa ONNX wasm runtime artifact.
fn webArtifact() -> LocalEngineArtifact {
    LocalEngineArtifact {
        target: LocalPlatformTarget {
            platform: LocalPlatform::Web,
            architecture: LocalArchitecture::Wasm32,
        },
        delivery: LocalEngineDelivery::Embedded,
        url: format!(
            "{SHERPA_ONNX_WEB_ASSET_BASE_URL}/sherpa-onnx-wasm-simd-v{SHERPA_ONNX_ENGINE_VERSION}-vad.tar.bz2"
        ),
        sha256: "7c4f2260a98f5d3e00275eb6bd012f15b29f687d22e9f33329b3a333e6843974"
            .to_string(),
        byteSize: 3_259_459,
        archiveFormat: LocalEngineArchiveFormat::TarBz2,
        archiveRoot: format!("sherpa-onnx-wasm-simd-v{SHERPA_ONNX_ENGINE_VERSION}-vad"),
        sttExecutable: None,
        ttsExecutable: None,
        androidLibraryDir: None,
        ohosLibraryDir: None,
        iosFrameworkDir: None,
        webRuntimeDir: Some(".".to_string()),
    }
}

/// Builds one Android Sherpa ONNX JNI archive artifact.
fn androidArtifact(architecture: LocalArchitecture, androidAbi: &str) -> LocalEngineArtifact {
    LocalEngineArtifact {
        target: LocalPlatformTarget {
            platform: LocalPlatform::Android,
            architecture,
        },
        delivery: LocalEngineDelivery::DownloadArchive,
        url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{SHERPA_ONNX_ENGINE_VERSION}/sherpa-onnx-v{SHERPA_ONNX_ENGINE_VERSION}-android.tar.bz2"
        ),
        sha256: "fc4d17941152941a883b0cfabfc9acac118682324e9f97df6c1ae1360bc7bc8e"
            .to_string(),
        byteSize: 52_236_308,
        archiveFormat: LocalEngineArchiveFormat::TarBz2,
        archiveRoot: ".".to_string(),
        sttExecutable: None,
        ttsExecutable: None,
        androidLibraryDir: Some(format!("jniLibs/{androidAbi}")),
        ohosLibraryDir: None,
        iosFrameworkDir: None,
        webRuntimeDir: None,
    }
}

/// Builds one OHOS Sherpa ONNX C API archive artifact.
fn ohosArtifact(
    architecture: LocalArchitecture,
    ohosAbi: &str,
    sha256: &str,
    byteSize: u64,
) -> LocalEngineArtifact {
    let archiveRoot = format!("sherpa-onnx-v{SHERPA_ONNX_ENGINE_VERSION}-ohos-{ohosAbi}");
    LocalEngineArtifact {
        target: LocalPlatformTarget {
            platform: LocalPlatform::Ohos,
            architecture,
        },
        delivery: LocalEngineDelivery::DownloadArchive,
        url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{SHERPA_ONNX_ENGINE_VERSION}/{archiveRoot}.tar.bz2"
        ),
        sha256: sha256.to_string(),
        byteSize,
        archiveFormat: LocalEngineArchiveFormat::TarBz2,
        archiveRoot,
        sttExecutable: None,
        ttsExecutable: None,
        androidLibraryDir: None,
        ohosLibraryDir: Some("lib".to_string()),
        iosFrameworkDir: None,
        webRuntimeDir: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies every engine artifact has one exact target and checksum.
    #[test]
    fn sherpaOnnxArtifactsAreTargetedAndVerified() {
        let manifest = LocalEngineCatalog::sherpaOnnx();
        assert_eq!(manifest.artifacts.len(), 15);
        for artifact in manifest.artifacts {
            assert_eq!(artifact.sha256.len(), 64);
            assert!(artifact.byteSize > 0);
        }
    }

    /// Verifies application-hosted engines are not downloaded during model installation.
    #[test]
    fn applicationHostedTargetsDeclareEmbeddedDelivery() {
        let manifest = LocalEngineCatalog::sherpaOnnx();
        for platform in [LocalPlatform::Ios, LocalPlatform::Web] {
            assert!(manifest
                .artifacts
                .iter()
                .filter(|artifact| artifact.target.platform == platform)
                .all(|artifact| artifact.delivery == LocalEngineDelivery::Embedded));
        }
    }
}
