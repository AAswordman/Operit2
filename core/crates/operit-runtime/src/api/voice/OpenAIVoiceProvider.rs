#![allow(non_snake_case)]

use serde_json::json;

use operit_host_api::HttpRequestData;

use crate::api::voice::VoiceService::VoiceService;
use operit_context::OperitApplicationContext::defaultHttpHost;
use crate::data::model::TtsConfig::TtsConfig;

pub struct OpenAIVoiceProvider;

impl OpenAIVoiceProvider {
    pub fn new() -> Self {
        Self
    }
}

impl VoiceService for OpenAIVoiceProvider {
    fn synthesize(&self, config: &TtsConfig, text: &str) -> Result<Vec<u8>, String> {
        let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        if !config.apiKey.trim().is_empty() {
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", config.apiKey.trim()),
            ));
        }
        let body = serde_json::to_vec(&json!({
            "model": config.model,
            "voice": config.voice,
            "input": text,
            "response_format": config.responseFormat,
            "speed": config.speed,
        }))
        .map_err(|error| error.to_string())?;
        let response = defaultHttpHost()
            .executeHttpRequest(HttpRequestData {
                url: config.endpoint.clone(),
                method: "POST".to_string(),
                headers,
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 120,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| error.to_string())?;
        if response.statusCode < 200 || response.statusCode >= 300 {
            let body = String::from_utf8_lossy(&response.body);
            return Err(format!(
                "tts request failed with status {}: {body}",
                response.statusCode
            ));
        }
        Ok(response.body)
    }
}
