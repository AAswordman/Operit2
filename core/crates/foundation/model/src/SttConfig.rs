use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct SttHttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct SttConfig {
    pub id: String,
    pub name: String,
    pub providerType: String,
    pub endpoint: String,
    pub apiKey: String,
    pub model: String,
    pub fileFieldName: String,
    pub modelFieldName: String,
    pub languageFieldName: String,
    pub responseTextJsonPath: String,
    pub headers: Vec<SttHttpHeader>,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct AvailableSttModel {
    pub model: String,
    pub displayName: String,
    pub description: String,
    pub languages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct SttProviderCatalogEntry {
    pub providerTypeId: String,
    pub displayName: String,
    pub defaultEndpoint: String,
    pub defaultModel: String,
    pub defaultFileFieldName: String,
    pub defaultModelFieldName: String,
    pub defaultLanguageFieldName: String,
    pub defaultResponseTextJsonPath: String,
    pub defaultHeaders: Vec<SttHttpHeader>,
}

pub struct SttProviderType;

impl SttProviderType {
    pub const LOCAL_MODEL: &'static str = "LOCAL_MODEL";
    pub const OPENAI_COMPATIBLE: &'static str = "OPENAI_COMPATIBLE";
    pub const HTTP_STT: &'static str = "HTTP_STT";

    /// Normalizes one STT provider type identifier.
    pub fn normalize(providerType: &str) -> String {
        providerType.trim().to_ascii_uppercase()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_snake_case)]
pub struct SttRecognitionResult {
    pub text: String,
}
