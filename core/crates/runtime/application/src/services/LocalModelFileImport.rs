use std::collections::BTreeSet;

use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_local_models::LocalEngineManifest::LocalEngineManifest;
use operit_local_models::LocalModelManifest::{
    LocalEngineKind, LocalEngineRequirement, LocalModelDriver, LocalModelFile,
    LocalModelInstallSource, LocalModelKind, LocalModelManifest,
};
use operit_local_models::LocalModelStorage::{buildLocalModelStoragePath, validatedStorageSegment};
use operit_util::stream::ReverseStream::ReverseStream;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{errorString, LocalModelImportFile};

const GGUF_FILE: &str = "model.gguf";
const GGUF_HEADER_BYTES: usize = 24;
const MNN_CONFIG: &str = "config.json";
const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 1024 * 1024;

pub(super) struct LocalModelFileImport {
    manifest: LocalModelManifest,
    inputs: Vec<LocalModelImportFile>,
    pub(super) storagePath: String,
}

impl LocalModelFileImport {
    /// Validates the selection and allocates its own runtime storage directory.
    pub(super) fn new(
        engine: LocalEngineKind,
        engineManifest: &LocalEngineManifest,
        displayName: String,
        mut files: Vec<LocalModelImportFile>,
    ) -> Result<Self, String> {
        let displayName = displayName.trim().to_string();
        if displayName.is_empty() || displayName.chars().any(char::is_control) {
            return Err("Enter a valid local model name".into());
        }
        if files.iter().any(|file| file.byteLength == 0) {
            return Err("Select non-empty local model files".into());
        }
        let driver = match engine {
            LocalEngineKind::LlamaCpp => {
                let [file] = files.as_mut_slice() else {
                    return Err("Select one complete .gguf model file".into());
                };
                let name = file.name.to_ascii_lowercase();
                let Some(stem) = name.strip_suffix(".gguf") else {
                    return Err("Select a .gguf model file".into());
                };
                if stem.is_empty()
                    || isGgufShard(stem)
                    || stem == "mmproj"
                    || stem.starts_with("mmproj-")
                {
                    return Err(
                        "Select a complete GGUF chat model without shards or projectors".into(),
                    );
                }
                if file.byteLength < GGUF_HEADER_BYTES as u64 {
                    return Err("The GGUF model header is incomplete".into());
                }
                file.name = GGUF_FILE.to_string();
                LocalModelDriver::LlamaCpp {
                    model: file.name.clone(),
                }
            }
            LocalEngineKind::Mnn => {
                let mut names = BTreeSet::new();
                for file in &files {
                    validateMnnReference(&file.name)?;
                    if !names.insert(file.name.to_ascii_lowercase()) {
                        return Err(format!("Repeated MNN bundle file: {}", file.name));
                    }
                }
                if !files.iter().any(|file| file.name == MNN_CONFIG) {
                    return Err("Select all MNN model files, including config.json".into());
                }
                LocalModelDriver::Mnn {
                    config: MNN_CONFIG.to_string(),
                }
            }
            _ => return Err("This local engine does not support file imports".into()),
        };
        let manifest = LocalModelManifest {
            id: format!("local-file-{}", Uuid::new_v4().simple()),
            version: "1".to_string(),
            displayName,
            description: String::new(),
            kind: LocalModelKind::Chat,
            engine,
            license: String::new(),
            homepage: String::new(),
            languages: Vec::new(),
            tags: Vec::new(),
            engineRequirement: Some(LocalEngineRequirement {
                engineId: engineManifest.id.clone(),
                version: engineManifest.version.clone(),
            }),
            driver: Some(driver),
            sources: Vec::new(),
            installSource: LocalModelInstallSource::Files,
            files: Vec::new(),
        };
        let storagePath = buildLocalModelStoragePath(
            &manifest.kind,
            &manifest.engine,
            &manifest.id,
            &manifest.version,
        )
        .map_err(errorString)?;
        Ok(Self {
            manifest,
            inputs: files,
            storagePath,
        })
    }

    /// Receives and checks every file before returning the manifest to register.
    pub(super) async fn receive(
        &self,
        storage: &dyn RuntimeStorageHost,
        writers: &dyn RuntimeStorageWriteHost,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<LocalModelManifest, String> {
        let mut manifest = self.manifest.clone();
        for file in &self.inputs {
            let path = format!("{}/{}", self.storagePath, file.name);
            manifest.files.push(
                receiveFile(
                    writers,
                    &path,
                    file,
                    manifest.engine == LocalEngineKind::LlamaCpp,
                    &mut bytes,
                )
                .await?,
            );
        }
        if !matches!(bytes.recv().await, Some(end) if end.is_empty())
            || bytes.recv().await.is_some()
        {
            return Err("Local model input did not complete cleanly".into());
        }
        if manifest.engine == LocalEngineKind::Mnn {
            self.validateMnnBundle(storage)?;
        }
        Ok(manifest)
    }

    /// Checks the required files of a flat text-only MNN model export.
    fn validateMnnBundle(&self, storage: &dyn RuntimeStorageHost) -> Result<(), String> {
        let mut config = self.readMnnConfig(storage, MNN_CONFIG)?;
        let modelConfigName = mnnFileName(&config, "llm_config", "llm_config.json")?;
        let modelConfig = self.readMnnConfig(storage, modelConfigName)?;
        for entry in [&config, &modelConfig] {
            if let Some(base) = entry.get("base_dir") {
                if !matches!(base.as_str(), Some("." | "./" | ".\\")) {
                    return Err("MNN base_dir must refer to the selected model files".into());
                }
            }
        }
        config.extend(modelConfig);
        for flag in ["is_visual", "is_audio", "has_talker"] {
            if config.get(flag).and_then(Value::as_bool) == Some(true) {
                return Err("Local MNN imports support text chat models".into());
            }
        }
        for (key, defaultName) in [
            ("llm_model", "llm.mnn"),
            ("llm_weight", "llm.mnn.weight"),
            ("tokenizer_file", "tokenizer.txt"),
        ] {
            self.requireInput(mnnFileName(&config, key, defaultName)?)?;
        }
        let tied = match config.get("tie_embeddings") {
            Some(Value::Array(parts)) if parts.len() >= 5 => {
                parts[0].as_i64().is_some_and(|offset| offset > 0)
            }
            Some(Value::Object(info)) => info
                .get("weight_offset")
                .and_then(Value::as_i64)
                .is_some_and(|offset| offset > 0),
            _ => false,
        };
        if !tied {
            self.requireInput(mnnFileName(
                &config,
                "embedding_file",
                "embeddings_bf16.bin",
            )?)?;
        } else if let Some(name) = config.get("embedding_file") {
            let name = name
                .as_str()
                .ok_or("MNN embedding_file must be a filename")?;
            validateMnnReference(name.trim_start_matches("./"))?;
        }
        Ok(())
    }

    /// Reads small MNN metadata from the just-imported files.
    fn readMnnConfig(
        &self,
        storage: &dyn RuntimeStorageHost,
        name: &str,
    ) -> Result<Map<String, Value>, String> {
        let file = self.requireInput(name)?;
        if file.byteLength > MAX_CONFIG_BYTES {
            return Err(format!("MNN configuration is too large: {name}"));
        }
        let bytes = storage
            .readBytes(&format!("{}/{name}", self.storagePath))
            .map_err(errorString)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("Invalid MNN configuration {name}: {error}"))
    }

    /// Resolves one required file by its exact name in the selected flat bundle.
    fn requireInput(&self, name: &str) -> Result<&LocalModelImportFile, String> {
        self.inputs
            .iter()
            .find(|file| file.name == name)
            .ok_or_else(|| format!("MNN model file is missing: {name}"))
    }
}

/// Copies one declared file in bounded chunks and records the resulting hash.
async fn receiveFile(
    writers: &dyn RuntimeStorageWriteHost,
    path: &str,
    file: &LocalModelImportFile,
    checkGguf: bool,
    bytes: &mut ReverseStream<Vec<u8>>,
) -> Result<LocalModelFile, String> {
    let mut writer = writers.createWriteSession(path).map_err(errorString)?;
    let result = async {
        let mut remaining = file.byteLength;
        let mut hasher = Sha256::new();
        let mut header = Vec::new();
        while remaining > 0 {
            let chunk = bytes
                .recv()
                .await
                .ok_or_else(|| format!("Incomplete local model file: {}", file.name))?;
            let length = chunk.len();
            if length == 0 || length > MAX_CHUNK_BYTES || length as u64 > remaining {
                return Err(format!(
                    "Invalid byte count for local model file: {}",
                    file.name
                ));
            }
            if checkGguf && header.len() < GGUF_HEADER_BYTES {
                let needed = GGUF_HEADER_BYTES - header.len();
                header.extend_from_slice(&chunk[..needed.min(length)]);
                if header.len() == GGUF_HEADER_BYTES {
                    validateGgufHeader(&header)?;
                }
            }
            writer.writeChunk(&chunk).map_err(errorString)?;
            hasher.update(&chunk);
            remaining -= length as u64;
        }
        Ok(LocalModelFile {
            relativePath: file.name.clone(),
            sha256: format!("{:x}", hasher.finalize()),
            byteSize: file.byteLength,
            sourceId: String::new(),
        })
    }
    .await;
    match result {
        Ok(file) => {
            writer.commit().map_err(errorString)?;
            Ok(file)
        }
        Err(error) => {
            if let Err(cleanup) = writer.discard() {
                operit_host_api::logHostError("LocalModelImport", &cleanup.to_string());
            }
            Err(error)
        }
    }
}

/// Checks the magic, supported version, and presence of tensors in a GGUF header.
fn validateGgufHeader(header: &[u8]) -> Result<(), String> {
    let version = u32::from_le_bytes(header[4..8].try_into().map_err(errorString)?);
    let tensorCount = u64::from_le_bytes(header[8..16].try_into().map_err(errorString)?);
    if &header[..4] != b"GGUF" || !matches!(version, 2 | 3) || tensorCount == 0 {
        return Err("This file is not a supported GGUF model".into());
    }
    Ok(())
}

/// Recognizes llama.cpp's standard filename suffix for split weights.
fn isGgufShard(stem: &str) -> bool {
    let Some((prefix, total)) = stem.rsplit_once("-of-") else {
        return false;
    };
    let Some((_, index)) = prefix.rsplit_once('-') else {
        return false;
    };
    index.len() == 5
        && total.len() == 5
        && index
            .bytes()
            .chain(total.bytes())
            .all(|byte| byte.is_ascii_digit())
        && total != "00001"
}

/// Reads a filename from MNN's configuration schema.
fn mnnFileName<'a>(
    config: &'a Map<String, Value>,
    key: &str,
    standardName: &'a str,
) -> Result<&'a str, String> {
    let name = match config.get(key) {
        Some(value) => value
            .as_str()
            .ok_or_else(|| format!("MNN {key} must be a filename"))?,
        None => standardName,
    }
    .trim_start_matches("./");
    validateMnnReference(name)?;
    Ok(name)
}

/// Keeps native model references inside a flat imported bundle.
fn validateMnnReference(name: &str) -> Result<(), String> {
    if validatedStorageSegment("MNN model reference", name).map_err(errorString)? != name
        || name.starts_with('.')
        || name.ends_with('.')
    {
        return Err(format!(
            "MNN model paths must be plain bundle filenames: {name}"
        ));
    }
    Ok(())
}
