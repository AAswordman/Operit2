use std::sync::Arc;

use operit_host_api::{HttpHost, HttpRequestData};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::LocalModelManifest::{
    LocalEngineKind, LocalEngineRequirement, LocalModelDriver, LocalModelFile,
    LocalModelInstallSource, LocalModelKind, LocalModelManifest, LocalModelSource,
    LocalModelSourceKind,
};

const SHERPA_ONNX_ENGINE_ID: &str = "sherpa-onnx";
const SHERPA_ONNX_ENGINE_VERSION: &str = "1.13.2";
const VOCODER_SOURCE_ID: &str = "github-sherpa-onnx-vocoder-models";
const VOCODER_FILE_NAME: &str = "vocos-22khz-univ.onnx";
const VOCODER_SHA256: &str = "0574a135aa1db2de6e181050db2ec528496cacd4a4701fc5d7faf9f9804c0081";
const VOCODER_BYTE_SIZE: u64 = 53_884_024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelHubRepoSummary {
    pub repository: String,
    pub revision: String,
    pub displayName: String,
    pub description: String,
    pub sourceKind: LocalModelSourceKind,
    pub downloads: u64,
    pub likes: u64,
    pub tags: Vec<String>,
    pub homepage: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct HubRepoFileEntry {
    pub relativePath: String,
    pub byteSize: u64,
    pub sha256: String,
}

#[derive(Clone)]
pub struct LocalModelHubClient {
    httpHost: Arc<dyn HttpHost>,
}

impl LocalModelHubClient {
    /// Creates a Hub client backed by the application HTTP host.
    pub fn new(httpHost: Arc<dyn HttpHost>) -> Self {
        Self { httpHost }
    }

    /// Searches Hugging Face or ModelScope for model repositories matching the query.
    pub fn searchRepositories(
        &self,
        sourceKind: LocalModelSourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LocalModelHubRepoSummary>, String> {
        let query = query.trim();
        let limit = limit.clamp(1, 50);
        match sourceKind {
            LocalModelSourceKind::ModelScope => self.searchModelScope(query, limit),
            LocalModelSourceKind::HuggingFace => {
                self.searchHuggingFace("https://huggingface.co", sourceKind, query, limit)
            }
            LocalModelSourceKind::HfMirror => {
                self.searchHuggingFace("https://hf-mirror.com", sourceKind, query, limit)
            }
            LocalModelSourceKind::DirectHttp => {
                Err("DirectHttp source does not support online repository search".to_string())
            }
        }
    }

    /// Inspects one remote repository on Hugging Face or ModelScope and builds an installable manifest.
    pub fn inspectRepositoryManifest(
        &self,
        sourceKind: LocalModelSourceKind,
        repositoryInput: &str,
        revisionInput: Option<&str>,
    ) -> Result<LocalModelManifest, String> {
        let (detectedKind, repository, urlRevision) =
            parseRepositoryInput(repositoryInput, sourceKind)?;
        let requestedRevision = revisionInput
            .map(str::trim)
            .filter(|rev| !rev.is_empty())
            .or(urlRevision.as_deref());

        match detectedKind {
            LocalModelSourceKind::ModelScope => {
                let revision = requestedRevision.unwrap_or("master");
                self.inspectModelScopeRepo(&repository, revision)
            }
            LocalModelSourceKind::HuggingFace => {
                let revision = requestedRevision.unwrap_or("main");
                self.inspectHuggingFaceRepo(
                    "https://huggingface.co",
                    LocalModelSourceKind::HuggingFace,
                    &repository,
                    revision,
                )
            }
            LocalModelSourceKind::HfMirror => {
                let revision = requestedRevision.unwrap_or("main");
                self.inspectHuggingFaceRepo(
                    "https://hf-mirror.com",
                    LocalModelSourceKind::HfMirror,
                    &repository,
                    revision,
                )
            }
            LocalModelSourceKind::DirectHttp => Err(
                "DirectHttp source does not support automatic repository manifest inspection"
                    .to_string(),
            ),
        }
    }

    fn searchModelScope(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LocalModelHubRepoSummary>, String> {
        let body = serde_json::to_vec(&json!({
            "PageSize": limit,
            "PageNumber": 1,
            "SortBy": "Default",
            "Target": "",
            "SingleCriterion": [],
            "Name": query,
        }))
        .map_err(|error| error.to_string())?;
        let response = self.executeJsonRequest(
            "PUT",
            "https://modelscope.cn/api/v1/dolphin/models",
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Accept".to_string(), "application/json".to_string()),
            ],
            body,
        )?;
        parseModelScopeSearchResponse(&response)
    }

    fn searchHuggingFace(
        &self,
        host: &str,
        sourceKind: LocalModelSourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LocalModelHubRepoSummary>, String> {
        let encodedQuery = encodeUrlQueryComponent(query);
        let url = format!("{host}/api/models?search={encodedQuery}&limit={limit}");
        let response = self.executeJsonRequest(
            "GET",
            &url,
            vec![("Accept".to_string(), "application/json".to_string())],
            Vec::new(),
        )?;
        parseHuggingFaceSearchResponse(&response, sourceKind)
    }

    fn inspectModelScopeRepo(
        &self,
        repository: &str,
        revision: &str,
    ) -> Result<LocalModelManifest, String> {
        let encodedRev = encodeUrlQueryComponent(revision);
        let url = format!(
            "https://modelscope.cn/api/v1/models/{repository}/repo/files?Recursive=1&Revision={encodedRev}"
        );
        let response = self.executeJsonRequest(
            "GET",
            &url,
            vec![("Accept".to_string(), "application/json".to_string())],
            Vec::new(),
        )?;
        let files = parseModelScopeRepoFiles(&response)?;
        let sourceId = format!("modelscope-{}", sanitizeModelId(repository));
        let source = LocalModelSource {
            id: sourceId,
            kind: LocalModelSourceKind::ModelScope,
            repository: repository.to_string(),
            revision: revision.to_string(),
            baseUrl: format!("https://modelscope.cn/models/{repository}/resolve/{revision}"),
        };
        let homepage = format!("https://modelscope.cn/models/{repository}");
        buildManifestFromRepoFiles(
            repository,
            revision,
            &homepage,
            "community",
            vec!["zh".to_string(), "en".to_string()],
            vec!["modelscope".to_string(), "sherpa-onnx".to_string()],
            source,
            &files,
        )
    }

    fn inspectHuggingFaceRepo(
        &self,
        host: &str,
        sourceKind: LocalModelSourceKind,
        repository: &str,
        revision: &str,
    ) -> Result<LocalModelManifest, String> {
        let encodedRev = encodeUrlQueryComponent(revision);
        let treeUrl = format!("{host}/api/models/{repository}/tree/{encodedRev}?recursive=true");
        let treeResponse = self.executeJsonRequest(
            "GET",
            &treeUrl,
            vec![("Accept".to_string(), "application/json".to_string())],
            Vec::new(),
        )?;
        let files = parseHuggingFaceRepoFiles(&treeResponse)?;
        let sourceId = format!("huggingface-{}", sanitizeModelId(repository));
        let baseHost = match sourceKind {
            LocalModelSourceKind::HfMirror => "https://hf-mirror.com",
            _ => "https://huggingface.co",
        };
        let source = LocalModelSource {
            id: sourceId,
            kind: LocalModelSourceKind::HuggingFace,
            repository: repository.to_string(),
            revision: revision.to_string(),
            baseUrl: format!("{baseHost}/{repository}/resolve/{revision}"),
        };
        let homepage = format!("https://huggingface.co/{repository}");
        buildManifestFromRepoFiles(
            repository,
            revision,
            &homepage,
            "community",
            vec!["zh".to_string(), "en".to_string()],
            vec!["huggingface".to_string(), "sherpa-onnx".to_string()],
            source,
            &files,
        )
    }

    fn executeJsonRequest(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<Value, String> {
        let response = self
            .httpHost
            .executeHttpRequest(HttpRequestData {
                url: url.to_string(),
                method: method.to_string(),
                headers,
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 20,
                readTimeoutSeconds: 30,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| error.to_string())?;
        if response.statusCode < 200 || response.statusCode >= 300 {
            return Err(format!(
                "Hub API request failed ({} {}): HTTP {} {}",
                method, url, response.statusCode, response.statusMessage
            ));
        }
        serde_json::from_slice(&response.body)
            .map_err(|error| format!("Hub API JSON decode failed ({url}): {error}"))
    }
}

/// Parses a repository identifier or full Hugging Face / ModelScope URL into (sourceKind, owner/repo, optional revision).
pub fn parseRepositoryInput(
    rawInput: &str,
    defaultKind: LocalModelSourceKind,
) -> Result<(LocalModelSourceKind, String, Option<String>), String> {
    let trimmed = rawInput.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Repository identifier cannot be empty".to_string());
    }

    let (detectedKind, remainder) = if let Some(rest) = trimmed
        .strip_prefix("https://huggingface.co/")
        .or_else(|| trimmed.strip_prefix("http://huggingface.co/"))
    {
        (LocalModelSourceKind::HuggingFace, rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("https://hf-mirror.com/")
        .or_else(|| trimmed.strip_prefix("http://hf-mirror.com/"))
    {
        (LocalModelSourceKind::HfMirror, rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("https://modelscope.cn/models/")
        .or_else(|| trimmed.strip_prefix("https://www.modelscope.cn/models/"))
        .or_else(|| trimmed.strip_prefix("http://modelscope.cn/models/"))
        .or_else(|| trimmed.strip_prefix("http://www.modelscope.cn/models/"))
    {
        (LocalModelSourceKind::ModelScope, rest)
    } else {
        (defaultKind, trimmed)
    };

    let segments: Vec<&str> = remainder
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 {
        return Err(format!(
            "Invalid repository format '{rawInput}', expected 'owner/repo'"
        ));
    }
    let owner = segments[0].trim();
    let repo = segments[1].trim();
    if owner.is_empty() || repo.is_empty() {
        return Err(format!(
            "Invalid repository format '{rawInput}', expected 'owner/repo'"
        ));
    }
    let revision = if segments.len() >= 4 && (segments[2] == "tree" || segments[2] == "resolve") {
        Some(segments[3].to_string())
    } else {
        None
    };

    Ok((detectedKind, format!("{owner}/{repo}"), revision))
}

/// Parses ModelScope search JSON into repository summaries.
pub fn parseModelScopeSearchResponse(value: &Value) -> Result<Vec<LocalModelHubRepoSummary>, String> {
    let models = value
        .pointer("/Data/Model/Models")
        .and_then(Value::as_array)
        .ok_or_else(|| "ModelScope search response is missing Data.Model.Models".to_string())?;
    let mut summaries = Vec::new();
    for item in models {
        let owner = item.get("Path").and_then(Value::as_str).unwrap_or("").trim();
        let name = item.get("Name").and_then(Value::as_str).unwrap_or("").trim();
        if owner.is_empty() || name.is_empty() {
            continue;
        }
        let repository = format!("{owner}/{name}");
        let revision = item
            .get("Revision")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|rev| !rev.is_empty())
            .unwrap_or("master")
            .to_string();
        let displayName = item
            .get("ChineseName")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(name)
            .to_string();
        let description = item
            .get("Description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(&repository)
            .to_string();
        let downloads = item.get("Downloads").and_then(Value::as_u64).unwrap_or(0);
        let likes = item.get("Stars").and_then(Value::as_u64).unwrap_or(0);
        let tags = item
            .get("Tags")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        summaries.push(LocalModelHubRepoSummary {
            homepage: format!("https://modelscope.cn/models/{repository}"),
            repository,
            revision,
            displayName,
            description,
            sourceKind: LocalModelSourceKind::ModelScope,
            downloads,
            likes,
            tags,
        });
    }
    Ok(summaries)
}

/// Parses Hugging Face search JSON into repository summaries.
pub fn parseHuggingFaceSearchResponse(
    value: &Value,
    sourceKind: LocalModelSourceKind,
) -> Result<Vec<LocalModelHubRepoSummary>, String> {
    let models = value
        .as_array()
        .ok_or_else(|| "Hugging Face search response must be a JSON array".to_string())?;
    let mut summaries = Vec::new();
    for item in models {
        let repository = item
            .get("id")
            .or_else(|| item.get("modelId"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if repository.is_empty() || !repository.contains('/') {
            continue;
        }
        let displayName = repository
            .split('/')
            .nth(1)
            .unwrap_or(repository)
            .to_string();
        let pipelineTag = item
            .get("pipeline_tag")
            .and_then(Value::as_str)
            .unwrap_or("");
        let tags = item
            .get("tags")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let description = if !pipelineTag.is_empty() {
            format!("{repository} ({pipelineTag})")
        } else {
            repository.to_string()
        };
        let downloads = item.get("downloads").and_then(Value::as_u64).unwrap_or(0);
        let likes = item.get("likes").and_then(Value::as_u64).unwrap_or(0);
        summaries.push(LocalModelHubRepoSummary {
            homepage: format!("https://huggingface.co/{repository}"),
            repository: repository.to_string(),
            revision: "main".to_string(),
            displayName,
            description,
            sourceKind,
            downloads,
            likes,
            tags,
        });
    }
    Ok(summaries)
}

/// Parses ModelScope repository file tree JSON into file entries.
pub fn parseModelScopeRepoFiles(value: &Value) -> Result<Vec<HubRepoFileEntry>, String> {
    let files = value
        .pointer("/Data/Files")
        .and_then(Value::as_array)
        .ok_or_else(|| "ModelScope repository response is missing Data.Files".to_string())?;
    let mut entries = Vec::new();
    for item in files {
        let itemType = item.get("Type").and_then(Value::as_str).unwrap_or("");
        if itemType != "blob" {
            continue;
        }
        let path = item
            .get("Path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if path.is_empty() {
            continue;
        }
        let byteSize = item.get("Size").and_then(Value::as_u64).unwrap_or(0);
        let sha256 = item
            .get("Sha256")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        entries.push(HubRepoFileEntry {
            relativePath: path.to_string(),
            byteSize,
            sha256,
        });
    }
    Ok(entries)
}

/// Parses Hugging Face repository tree JSON into file entries.
pub fn parseHuggingFaceRepoFiles(value: &Value) -> Result<Vec<HubRepoFileEntry>, String> {
    let files = value
        .as_array()
        .ok_or_else(|| "Hugging Face tree response must be a JSON array".to_string())?;
    let mut entries = Vec::new();
    for item in files {
        let itemType = item.get("type").and_then(Value::as_str).unwrap_or("");
        if itemType != "file" {
            continue;
        }
        let path = item
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if path.is_empty() {
            continue;
        }
        let lfsSize = item.pointer("/lfs/size").and_then(Value::as_u64);
        let byteSize = lfsSize
            .or_else(|| item.get("size").and_then(Value::as_u64))
            .unwrap_or(0);
        let sha256 = item
            .pointer("/lfs/oid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|oid| oid.len() == 64)
            .unwrap_or("")
            .to_string();
        entries.push(HubRepoFileEntry {
            relativePath: path.to_string(),
            byteSize,
            sha256,
        });
    }
    Ok(entries)
}

/// Infers the local model driver and builds a complete manifest from repository files.
pub fn buildManifestFromRepoFiles(
    repository: &str,
    revision: &str,
    homepage: &str,
    license: &str,
    languages: Vec<String>,
    tags: Vec<String>,
    primarySource: LocalModelSource,
    repoFiles: &[HubRepoFileEntry],
) -> Result<LocalModelManifest, String> {
    let modelId = sanitizeModelId(repository.split('/').nth(1).unwrap_or(repository));
    let displayName = repository
        .split('/')
        .nth(1)
        .unwrap_or(repository)
        .to_string();
    let sourceId = primarySource.id.clone();
    let mut sources = vec![primarySource.clone()];
    if primarySource.kind == LocalModelSourceKind::HuggingFace
        || primarySource.kind == LocalModelSourceKind::HfMirror
    {
        sources[0].kind = LocalModelSourceKind::HuggingFace;
        sources[0].baseUrl = format!("https://huggingface.co/{repository}/resolve/{revision}");
    }

    let rootOnnxFiles: Vec<&HubRepoFileEntry> = repoFiles
        .iter()
        .filter(|f| !f.relativePath.contains('/') && f.relativePath.ends_with(".onnx"))
        .collect();
    let tokensFile = findRepoFile(repoFiles, "tokens.txt");

    // 1. Streaming Transducer STT
    let encoderFile = selectPreferredOnnx(&rootOnnxFiles, "encoder", true);
    let decoderFile = selectPreferredOnnx(&rootOnnxFiles, "decoder", false);
    let joinerFile = selectPreferredOnnx(&rootOnnxFiles, "joiner", true);
    if let (Some(encoder), Some(decoder), Some(joiner), Some(tokens)) =
        (encoderFile, decoderFile, joinerFile, tokensFile)
    {
        let selectedFiles = vec![
            toModelFile(&sourceId, encoder),
            toModelFile(&sourceId, decoder),
            toModelFile(&sourceId, joiner),
            toModelFile(&sourceId, tokens),
        ];
        return Ok(LocalModelManifest {
            id: modelId,
            version: revision.to_string(),
            displayName,
            description: format!("Imported Sherpa ONNX Streaming STT ({repository})"),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaOnnx,
            license: license.to_string(),
            homepage: homepage.to_string(),
            languages,
            tags,
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxStreamingTransducer {
                encoder: encoder.relativePath.clone(),
                decoder: decoder.relativePath.clone(),
                joiner: joiner.relativePath.clone(),
                tokens: tokens.relativePath.clone(),
                modelType: "zipformer".to_string(),
            }),
            sources,
            installSource: LocalModelInstallSource::Files,
            files: selectedFiles,
        });
    }

    // 2. Kitten TTS
    let voicesFile = findRepoFile(repoFiles, "voices.bin");
    let kittenModelFile = selectPreferredOnnx(&rootOnnxFiles, "model", true);
    let hasEspeakDir = repoFiles
        .iter()
        .any(|f| f.relativePath.starts_with("espeak-ng-data/"));
    if let (Some(model), Some(voices), Some(tokens), true) =
        (kittenModelFile, voicesFile, tokensFile, hasEspeakDir)
    {
        let mut selectedFiles = vec![
            toModelFile(&sourceId, model),
            toModelFile(&sourceId, voices),
            toModelFile(&sourceId, tokens),
        ];
        for entry in repoFiles {
            if entry.relativePath.starts_with("espeak-ng-data/") && entry.byteSize > 0 {
                selectedFiles.push(toModelFile(&sourceId, entry));
            }
        }
        return Ok(LocalModelManifest {
            id: modelId,
            version: revision.to_string(),
            displayName,
            description: format!("Imported Sherpa ONNX Kitten TTS ({repository})"),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: license.to_string(),
            homepage: homepage.to_string(),
            languages,
            tags,
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxKitten {
                model: model.relativePath.clone(),
                voices: voices.relativePath.clone(),
                tokens: tokens.relativePath.clone(),
                dataDir: "espeak-ng-data".to_string(),
                speakerCount: 8,
            }),
            sources,
            installSource: LocalModelInstallSource::Files,
            files: selectedFiles,
        });
    }

    // 3. Matcha TTS
    let lexiconFile = findRepoFile(repoFiles, "lexicon.txt");
    let matchaAcoustic = rootOnnxFiles.iter().copied().find(|f| {
        f.relativePath.starts_with("model-steps-") || f.relativePath.contains("matcha")
    });
    if let (Some(acoustic), Some(lexicon), Some(tokens)) =
        (matchaAcoustic, lexiconFile, tokensFile)
    {
        let mut selectedFiles = vec![
            toModelFile(&sourceId, acoustic),
            toModelFile(&sourceId, tokens),
            toModelFile(&sourceId, lexicon),
        ];
        let mut ruleFsts = Vec::new();
        for fstName in ["phone.fst", "number.fst", "date.fst", "new_heteronym.fst"] {
            if let Some(fst) = findRepoFile(repoFiles, fstName) {
                ruleFsts.push(fst.relativePath.clone());
                selectedFiles.push(toModelFile(&sourceId, fst));
            }
        }
        let mut ruleFars = Vec::new();
        if let Some(far) = findRepoFile(repoFiles, "rule.far") {
            ruleFars.push(far.relativePath.clone());
            selectedFiles.push(toModelFile(&sourceId, far));
        }
        let vocoderFileName = if let Some(vocoder) = rootOnnxFiles
            .iter()
            .copied()
            .find(|f| f.relativePath.starts_with("vocos-") || f.relativePath.contains("vocoder"))
        {
            selectedFiles.push(toModelFile(&sourceId, vocoder));
            vocoder.relativePath.clone()
        } else {
            sources.push(LocalModelSource {
                id: VOCODER_SOURCE_ID.to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "k2-fsa/sherpa-onnx".to_string(),
                revision: "vocoder-models".to_string(),
                baseUrl: "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models"
                    .to_string(),
            });
            selectedFiles.push(LocalModelFile {
                relativePath: VOCODER_FILE_NAME.to_string(),
                sha256: VOCODER_SHA256.to_string(),
                byteSize: VOCODER_BYTE_SIZE,
                sourceId: VOCODER_SOURCE_ID.to_string(),
            });
            VOCODER_FILE_NAME.to_string()
        };

        return Ok(LocalModelManifest {
            id: modelId,
            version: revision.to_string(),
            displayName,
            description: format!("Imported Sherpa ONNX Matcha TTS ({repository})"),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: license.to_string(),
            homepage: homepage.to_string(),
            languages,
            tags,
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxMatcha {
                acousticModel: acoustic.relativePath.clone(),
                vocoder: vocoderFileName,
                lexicon: lexicon.relativePath.clone(),
                tokens: tokens.relativePath.clone(),
                ruleFsts,
                ruleFars,
                speakerCount: 1,
            }),
            sources,
            installSource: LocalModelInstallSource::Files,
            files: selectedFiles,
        });
    }

    // 4. VITS TTS
    let vitsModel = selectPreferredOnnx(&rootOnnxFiles, "vits", true)
        .or_else(|| selectPreferredOnnx(&rootOnnxFiles, "model", true));
    if let (Some(model), Some(lexicon), Some(tokens)) = (vitsModel, lexiconFile, tokensFile) {
        let mut selectedFiles = vec![
            toModelFile(&sourceId, model),
            toModelFile(&sourceId, lexicon),
            toModelFile(&sourceId, tokens),
        ];
        let mut ruleFsts = Vec::new();
        for fstName in ["phone.fst", "number.fst", "date.fst", "new_heteronym.fst"] {
            if let Some(fst) = findRepoFile(repoFiles, fstName) {
                ruleFsts.push(fst.relativePath.clone());
                selectedFiles.push(toModelFile(&sourceId, fst));
            }
        }
        let mut ruleFars = Vec::new();
        if let Some(far) = findRepoFile(repoFiles, "rule.far") {
            ruleFars.push(far.relativePath.clone());
            selectedFiles.push(toModelFile(&sourceId, far));
        }
        return Ok(LocalModelManifest {
            id: modelId,
            version: revision.to_string(),
            displayName,
            description: format!("Imported Sherpa ONNX VITS TTS ({repository})"),
            kind: LocalModelKind::TextToSpeech,
            engine: LocalEngineKind::SherpaOnnx,
            license: license.to_string(),
            homepage: homepage.to_string(),
            languages,
            tags,
            engineRequirement: Some(sherpaOnnxRequirement()),
            driver: Some(LocalModelDriver::SherpaOnnxVits {
                model: model.relativePath.clone(),
                lexicon: lexicon.relativePath.clone(),
                tokens: tokens.relativePath.clone(),
                ruleFsts,
                ruleFars,
                speakerCount: 1,
            }),
            sources,
            installSource: LocalModelInstallSource::Files,
            files: selectedFiles,
        });
    }

    Err(format!(
        "Repository '{repository}' does not match any supported Sherpa ONNX STT/TTS model layout"
    ))
}

fn findRepoFile<'a>(
    files: &'a [HubRepoFileEntry],
    relativePath: &str,
) -> Option<&'a HubRepoFileEntry> {
    files.iter().find(|f| f.relativePath == relativePath)
}

fn selectPreferredOnnx<'a>(
    onnxFiles: &[&'a HubRepoFileEntry],
    keyword: &str,
    preferInt8: bool,
) -> Option<&'a HubRepoFileEntry> {
    let matching: Vec<&'a HubRepoFileEntry> = onnxFiles
        .iter()
        .copied()
        .filter(|f| f.relativePath.to_ascii_lowercase().contains(keyword))
        .collect();
    if matching.is_empty() {
        return None;
    }
    if preferInt8 {
        if let Some(int8) = matching
            .iter()
            .copied()
            .find(|f| f.relativePath.ends_with(".int8.onnx"))
        {
            return Some(int8);
        }
    } else if let Some(nonInt8) = matching
        .iter()
        .copied()
        .find(|f| !f.relativePath.ends_with(".int8.onnx"))
    {
        return Some(nonInt8);
    }
    Some(matching[0])
}

fn toModelFile(sourceId: &str, entry: &HubRepoFileEntry) -> LocalModelFile {
    LocalModelFile {
        relativePath: entry.relativePath.clone(),
        sha256: entry.sha256.clone(),
        byteSize: entry.byteSize,
        sourceId: sourceId.to_string(),
    }
}

fn sherpaOnnxRequirement() -> LocalEngineRequirement {
    LocalEngineRequirement {
        engineId: SHERPA_ONNX_ENGINE_ID.to_string(),
        version: SHERPA_ONNX_ENGINE_VERSION.to_string(),
    }
}

fn sanitizeModelId(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn encodeUrlQueryComponent(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsesRepositoryUrlsAndShorthand() {
        let (kind, repo, rev) = parseRepositoryInput(
            "https://modelscope.cn/models/liaowenbin/sherpa-onnx-vits-zh-ll/summary",
            LocalModelSourceKind::HuggingFace,
        )
        .unwrap();
        assert_eq!(kind, LocalModelSourceKind::ModelScope);
        assert_eq!(repo, "liaowenbin/sherpa-onnx-vits-zh-ll");
        assert_eq!(rev, None);

        let (kind2, repo2, rev2) = parseRepositoryInput(
            "https://huggingface.co/csukuangfj/vits-zh-aishell3/tree/main",
            LocalModelSourceKind::ModelScope,
        )
        .unwrap();
        assert_eq!(kind2, LocalModelSourceKind::HuggingFace);
        assert_eq!(repo2, "csukuangfj/vits-zh-aishell3");
        assert_eq!(rev2.as_deref(), Some("main"));
    }

    #[test]
    fn infersStreamingTransducerAndVitsManifestsFromRepoFiles() {
        let source = LocalModelSource {
            id: "ms-test".to_string(),
            kind: LocalModelSourceKind::ModelScope,
            repository: "pkufool/test-zipformer".to_string(),
            revision: "master".to_string(),
            baseUrl: "https://modelscope.cn/models/pkufool/test-zipformer/resolve/master"
                .to_string(),
        };
        let sttFiles = vec![
            HubRepoFileEntry {
                relativePath: "encoder-epoch-99.int8.onnx".to_string(),
                byteSize: 100,
                sha256: "a".repeat(64),
            },
            HubRepoFileEntry {
                relativePath: "decoder-epoch-99.onnx".to_string(),
                byteSize: 50,
                sha256: "b".repeat(64),
            },
            HubRepoFileEntry {
                relativePath: "joiner-epoch-99.int8.onnx".to_string(),
                byteSize: 25,
                sha256: "c".repeat(64),
            },
            HubRepoFileEntry {
                relativePath: "tokens.txt".to_string(),
                byteSize: 10,
                sha256: "d".repeat(64),
            },
        ];
        let manifest = buildManifestFromRepoFiles(
            "pkufool/test-zipformer",
            "master",
            "https://modelscope.cn/models/pkufool/test-zipformer",
            "apache-2.0",
            vec!["zh".to_string()],
            vec!["stt".to_string()],
            source,
            &sttFiles,
        )
        .unwrap();
        assert_eq!(manifest.kind, LocalModelKind::SpeechToText);
        assert_eq!(manifest.files.len(), 4);
    }
}
