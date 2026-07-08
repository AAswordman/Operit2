use serde::{de::Error, Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsHttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsHttpResponsePipelineStep {
    #[serde(alias = "type")]
    pub stepType: String,
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_tts_http_headers")]
    pub headers: Vec<TtsHttpHeader>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsConfig {
    pub id: String,
    pub name: String,
    pub providerType: String,
    pub endpoint: String,
    pub apiKey: String,
    pub model: String,
    pub voice: String,
    pub responseFormat: String,
    pub speed: f64,
    #[serde(default = "default_tts_http_method")]
    pub httpMethod: String,
    #[serde(default)]
    pub requestBody: String,
    #[serde(default = "default_tts_content_type")]
    pub contentType: String,
    #[serde(default, deserialize_with = "deserialize_tts_http_headers")]
    pub headers: Vec<TtsHttpHeader>,
    #[serde(default)]
    pub responsePipeline: Vec<TtsHttpResponsePipelineStep>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct AvailableTtsVoice {
    pub model: String,
    pub voice: String,
    pub displayName: String,
    pub description: String,
    pub responseFormat: String,
    pub speed: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsProviderOperationResultSpec {
    pub itemsJsonPath: Option<String>,
    pub modelJsonPath: Option<String>,
    pub voiceJsonPath: Option<String>,
    pub displayNameJsonPath: Option<String>,
    pub descriptionJsonPath: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsProviderOperationSpec {
    pub operationType: String,
    pub handlerId: String,
    pub method: String,
    pub path: String,
    pub requiresApiKey: bool,
    pub authHeaderName: String,
    pub authHeaderValue: String,
    pub body: String,
    pub result: TtsProviderOperationResultSpec,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsProviderCatalogEntry {
    pub providerTypeId: String,
    pub displayName: String,
    pub defaultEndpoint: String,
    pub defaultModel: String,
    pub defaultResponseFormat: String,
    pub defaultHttpMethod: String,
    pub defaultContentType: String,
    pub defaultRequestBody: String,
    pub defaultHeaders: Vec<TtsHttpHeader>,
    pub defaultResponsePipeline: Vec<TtsHttpResponsePipelineStep>,
    pub operations: Vec<TtsProviderOperationSpec>,
}

pub struct TtsProviderType;

impl TtsProviderType {
    pub const SYSTEM_TTS: &'static str = "SYSTEM_TTS";
    pub const HTTP_TTS: &'static str = "HTTP_TTS";

    pub fn normalize(providerType: &str) -> String {
        providerType.trim().to_ascii_uppercase()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsSynthesisResult {
    pub audioPaths: Vec<String>,
    pub audioStoragePaths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsPlaybackResult {
    pub path: String,
    pub started: bool,
    pub details: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsHostPlaybackResult {
    pub path: String,
    pub active: bool,
    pub paused: bool,
    pub details: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct TtsPlaybackState {
    pub phase: String,
    pub currentText: String,
    pub currentAudioPath: String,
    pub queueLength: usize,
    pub audioIndex: usize,
    pub audioCount: usize,
    pub generation: u64,
    pub error: Option<String>,
}

fn default_tts_http_method() -> String {
    "POST".to_string()
}

fn default_tts_content_type() -> String {
    "application/json".to_string()
}

fn deserialize_tts_http_headers<'de, D>(deserializer: D) -> Result<Vec<TtsHttpHeader>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(|item| {
                serde_json::from_value::<TtsHttpHeader>(item).map_err(|error| {
                    D::Error::custom(format!("invalid tts http header item: {error}"))
                })
            })
            .collect(),
        Value::Object(entries) => Ok(entries
            .into_iter()
            .map(|(name, value)| TtsHttpHeader {
                name,
                value: match value {
                    Value::String(text) => text,
                    other => other.to_string(),
                },
            })
            .collect()),
        _ => Err(D::Error::custom("tts http headers must be array or object")),
    }
}
