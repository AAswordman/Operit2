#![allow(non_snake_case)]

use operit_host_api::HostManager::defaultHttpHost;
use operit_host_api::{HttpFilePart, HttpRequestData};
use operit_model::SttConfig::{SttConfig, SttRecognitionResult};
use serde_json::Value;

use crate::stt::SpeechToTextService::SpeechToTextService;

pub struct HttpSpeechToTextProvider;

impl HttpSpeechToTextProvider {
    /// Creates a multipart HTTP speech-to-text provider.
    pub fn new() -> Self {
        Self
    }
}

impl SpeechToTextService for HttpSpeechToTextProvider {
    /// Uploads one in-memory audio payload and extracts text from the configured JSON response path.
    fn transcribe(
        &self,
        config: &SttConfig,
        audioBytes: &[u8],
        fileName: &str,
        contentType: &str,
        language: Option<&str>,
    ) -> Result<SttRecognitionResult, String> {
        if audioBytes.is_empty() {
            return Err("STT audio payload is empty".to_string());
        }
        let fileName = fileName.trim();
        if fileName.is_empty() {
            return Err("STT audio file name is empty".to_string());
        }
        let contentType = contentType.trim();
        if contentType.is_empty() {
            return Err("STT audio content type is empty".to_string());
        }
        let mut formFields = vec![(config.modelFieldName.clone(), config.model.clone())];
        if let Some(language) = language {
            let language = language.trim();
            if !language.is_empty() {
                if config.languageFieldName.is_empty() {
                    return Err("STT provider does not declare a language field".to_string());
                }
                formFields.push((config.languageFieldName.clone(), language.to_string()));
            }
        }
        let headers = config
            .headers
            .iter()
            .map(|header| {
                (
                    header.name.clone(),
                    header.value.replace("{apiKey}", config.apiKey.trim()),
                )
            })
            .collect::<Vec<_>>();
        let response = defaultHttpHost()
            .executeHttpRequest(HttpRequestData {
                url: config.endpoint.clone(),
                method: "POST".to_string(),
                headers,
                body: Vec::new(),
                formFields,
                fileParts: vec![HttpFilePart {
                    fieldName: config.fileFieldName.clone(),
                    fileName: fileName.to_string(),
                    contentType: contentType.to_string(),
                    content: audioBytes.to_vec(),
                }],
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 180,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| error.to_string())?;
        if response.statusCode < 200 || response.statusCode >= 300 {
            let body = String::from_utf8_lossy(&response.body);
            return Err(format!(
                "STT request failed with status {}: {body}",
                response.statusCode
            ));
        }
        let json = serde_json::from_slice::<Value>(&response.body)
            .map_err(|error| format!("STT response is not valid JSON: {error}"))?;
        let text = selectJsonPath(&json, &config.responseTextJsonPath)?
            .as_str()
            .ok_or_else(|| "STT response text value is not a string".to_string())?;
        Ok(SttRecognitionResult {
            text: text.to_string(),
        })
    }
}

/// Selects one exact value from a JSON document using dot and array tokens.
fn selectJsonPath<'a>(value: &'a Value, path: &str) -> Result<&'a Value, String> {
    let path = path.trim();
    if path == "$" {
        return Ok(value);
    }
    let path = path
        .strip_prefix("$.")
        .ok_or_else(|| "STT response JSON path must start with $.".to_string())?;
    let mut current = value;
    for segment in path.split('.') {
        current = selectJsonSegment(current, segment)?;
    }
    Ok(current)
}

/// Selects one object key and its optional array indices.
fn selectJsonSegment<'a>(value: &'a Value, segment: &str) -> Result<&'a Value, String> {
    let keyEnd = match segment.find('[') {
        Some(index) => index,
        None => segment.len(),
    };
    let key = &segment[..keyEnd];
    if key.is_empty() {
        return Err("STT response JSON path key is empty".to_string());
    }
    let mut current = value
        .get(key)
        .ok_or_else(|| format!("STT response JSON path key is missing: {key}"))?;
    let mut rest = &segment[keyEnd..];
    while !rest.is_empty() {
        let end = rest
            .find(']')
            .ok_or_else(|| "STT response JSON path index is not closed".to_string())?;
        if !rest.starts_with('[') {
            return Err("STT response JSON path index is invalid".to_string());
        }
        let index = rest[1..end]
            .parse::<usize>()
            .map_err(|error| format!("invalid STT response JSON path index: {error}"))?;
        current = current
            .as_array()
            .ok_or_else(|| "STT response JSON path value is not an array".to_string())?
            .get(index)
            .ok_or_else(|| format!("STT response JSON path index is missing: {index}"))?;
        rest = &rest[end + 1..];
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::HostManager::setDefaultHttpHost;
    use operit_host_api::{
        HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback,
        HttpDownloadRequest, HttpDownloadResult, HttpHost, HttpResponseData,
    };
    use operit_model::SttConfig::{SttHttpHeader, SttProviderType};
    use std::sync::{Arc, Mutex};

    struct CapturingHttpHost {
        request: Arc<Mutex<Option<HttpRequestData>>>,
    }

    impl HttpHost for CapturingHttpHost {
        /// Captures one buffered HTTP request and returns a valid STT response.
        fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
            *self.request.lock().unwrap() = Some(request);
            Ok(HttpResponseData {
                finalUrl: "https://speech.example.test/transcribe".to_string(),
                statusCode: 200,
                statusMessage: "OK".to_string(),
                headers: Vec::new(),
                body: br#"{"text":"recognized"}"#.to_vec(),
            })
        }

        /// Rejects downloads because STT only sends buffered multipart requests.
        fn downloadFiles(
            &self,
            _request: HttpDownloadRequest,
            _control: HttpDownloadControl,
            _onProgress: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            Err(HostError::new("STT test host does not download files"))
        }
    }

    /// Creates one complete remote STT configuration for provider tests.
    fn testConfig() -> SttConfig {
        SttConfig {
            id: "test-stt".to_string(),
            name: "Test STT".to_string(),
            providerType: SttProviderType::HTTP_STT.to_string(),
            endpoint: "https://speech.example.test/transcribe".to_string(),
            apiKey: "secret".to_string(),
            model: "speech-model".to_string(),
            fileFieldName: "file".to_string(),
            modelFieldName: "model".to_string(),
            languageFieldName: "language".to_string(),
            responseTextJsonPath: "$.text".to_string(),
            headers: vec![SttHttpHeader {
                name: "Authorization".to_string(),
                value: "Bearer {apiKey}".to_string(),
            }],
            createdAt: 1,
            updatedAt: 1,
        }
    }

    /// Verifies STT response extraction follows exact object and array tokens.
    #[test]
    fn selectsConfiguredResponseTextPath() {
        let value = serde_json::json!({"results": [{"text": "hello"}]});
        let selected = selectJsonPath(&value, "$.results[0].text").unwrap();
        assert_eq!(selected.as_str(), Some("hello"));
    }

    /// Verifies in-memory audio metadata reaches the multipart HTTP host unchanged.
    #[test]
    fn uploadsInMemoryAudioPayload() {
        let captured = Arc::new(Mutex::new(None));
        setDefaultHttpHost(Arc::new(CapturingHttpHost {
            request: captured.clone(),
        }));
        let response = HttpSpeechToTextProvider::new()
            .transcribe(
                &testConfig(),
                &[0x52, 0x49, 0x46, 0x46],
                "voice.wav",
                "audio/wav",
                Some("zh"),
            )
            .unwrap();
        assert_eq!(response.text, "recognized");

        let request = captured.lock().unwrap().take().unwrap();
        assert_eq!(
            request.formFields,
            vec![
                ("model".to_string(), "speech-model".to_string()),
                ("language".to_string(), "zh".to_string()),
            ]
        );
        assert_eq!(
            request.headers,
            vec![("Authorization".to_string(), "Bearer secret".to_string())]
        );
        assert_eq!(request.fileParts.len(), 1);
        let file = &request.fileParts[0];
        assert_eq!(file.fieldName, "file");
        assert_eq!(file.fileName, "voice.wav");
        assert_eq!(file.contentType, "audio/wav");
        assert_eq!(file.content, vec![0x52, 0x49, 0x46, 0x46]);
    }
}
