use crate::SttConfig::{AvailableSttModel, SttHttpHeader, SttProviderCatalogEntry};

const STT_PROVIDER_ROWS: &str = r#"
LOCAL_MODEL|本地模型||||||$.text|[]
OPENAI_COMPATIBLE|OpenAI 兼容|https://api.openai.com/v1/audio/transcriptions|whisper-1|file|model|language|$.text|[{"name":"Authorization","value":"Bearer {apiKey}"}]
HTTP_STT|HTTP STT|||file|model|language|$.text|[]
GROQ_STT|Groq|https://api.groq.com/openai/v1/audio/transcriptions|whisper-large-v3-turbo|file|model|language|$.text|[{"name":"Authorization","value":"Bearer {apiKey}"}]
"#;

const STT_MODEL_ROWS: &str = r#"
OPENAI_COMPATIBLE|whisper-1|Whisper 1|OpenAI multilingual speech recognition|multilingual
OPENAI_COMPATIBLE|gpt-4o-transcribe|GPT-4o Transcribe|OpenAI speech recognition|multilingual
OPENAI_COMPATIBLE|gpt-4o-mini-transcribe|GPT-4o Mini Transcribe|OpenAI compact speech recognition|multilingual
GROQ_STT|whisper-large-v3-turbo|Whisper Large V3 Turbo|Groq multilingual speech recognition|multilingual
GROQ_STT|whisper-large-v3|Whisper Large V3|Groq multilingual speech recognition|multilingual
"#;

pub struct SttCatalog;

impl SttCatalog {
    /// Returns one STT provider catalog entry by exact provider type id.
    pub fn provider(providerTypeId: &str) -> Result<SttProviderCatalogEntry, String> {
        Self::providers()?
            .into_iter()
            .find(|provider| provider.providerTypeId.eq_ignore_ascii_case(providerTypeId))
            .ok_or_else(|| format!("stt catalog provider not found: {providerTypeId}"))
    }

    /// Returns every built-in STT provider catalog entry.
    pub fn providers() -> Result<Vec<SttProviderCatalogEntry>, String> {
        dataLines(STT_PROVIDER_ROWS).map(parseProviderRow).collect()
    }

    /// Returns built-in remote models declared for one STT provider type.
    pub fn modelsForProvider(providerTypeId: &str) -> Result<Vec<AvailableSttModel>, String> {
        let providerTypeId = providerTypeId.trim();
        dataLines(STT_MODEL_ROWS)
            .map(parseModelRow)
            .filter_map(|result| match result {
                Ok((rowProviderType, model)) if rowProviderType == providerTypeId => {
                    Some(Ok(model))
                }
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
}

/// Parses one provider catalog row.
fn parseProviderRow(line: &str) -> Result<SttProviderCatalogEntry, String> {
    let parts = line.split('|').collect::<Vec<_>>();
    if parts.len() != 9 {
        return Err(format!("invalid stt provider catalog row: {line}"));
    }
    Ok(SttProviderCatalogEntry {
        providerTypeId: parts[0].trim().to_string(),
        displayName: parts[1].trim().to_string(),
        defaultEndpoint: parts[2].trim().to_string(),
        defaultModel: parts[3].trim().to_string(),
        defaultFileFieldName: parts[4].trim().to_string(),
        defaultModelFieldName: parts[5].trim().to_string(),
        defaultLanguageFieldName: parts[6].trim().to_string(),
        defaultResponseTextJsonPath: parts[7].trim().to_string(),
        defaultHeaders: serde_json::from_str(parts[8].trim())
            .map_err(|error| format!("invalid stt provider headers: {error}"))?,
    })
}

/// Parses one provider model catalog row.
fn parseModelRow(line: &str) -> Result<(String, AvailableSttModel), String> {
    let parts = line.split('|').collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(format!("invalid stt model catalog row: {line}"));
    }
    Ok((
        parts[0].trim().to_string(),
        AvailableSttModel {
            model: parts[1].trim().to_string(),
            displayName: parts[2].trim().to_string(),
            description: parts[3].trim().to_string(),
            languages: parts[4]
                .split(',')
                .map(str::trim)
                .filter(|language| !language.is_empty())
                .map(str::to_string)
                .collect(),
        },
    ))
}

/// Returns non-empty trimmed rows from one catalog string.
fn dataLines(rows: &str) -> impl Iterator<Item = &str> {
    rows.lines().map(str::trim).filter(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies local and remote STT providers expose distinct catalog contracts.
    #[test]
    fn providersDeclareLocalAndMultipartRemoteEntries() {
        let local = SttCatalog::provider("LOCAL_MODEL").unwrap();
        let remote = SttCatalog::provider("OPENAI_COMPATIBLE").unwrap();
        assert!(local.defaultEndpoint.is_empty());
        assert_eq!(remote.defaultFileFieldName, "file");
        assert_eq!(remote.defaultModelFieldName, "model");
        assert_eq!(remote.defaultResponseTextJsonPath, "$.text");
    }

    /// Verifies remote model rows remain scoped to their provider type.
    #[test]
    fn remoteModelsAreProviderScoped() {
        let openai = SttCatalog::modelsForProvider("OPENAI_COMPATIBLE").unwrap();
        let groq = SttCatalog::modelsForProvider("GROQ_STT").unwrap();
        assert!(openai.iter().any(|model| model.model == "whisper-1"));
        assert!(groq
            .iter()
            .any(|model| model.model == "whisper-large-v3-turbo"));
    }
}
