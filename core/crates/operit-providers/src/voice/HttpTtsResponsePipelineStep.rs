#![allow(non_snake_case)]

pub struct HttpTtsResponsePipelineStepType;

impl HttpTtsResponsePipelineStepType {
    pub const PARSE_JSON: &'static str = "parse_json";
    pub const PICK: &'static str = "pick";
    pub const PARSE_JSON_STRING: &'static str = "parse_json_string";
    pub const HTTP_GET: &'static str = "http_get";
    pub const HTTP_REQUEST_FROM_OBJECT: &'static str = "http_request_from_object";
    pub const BASE64_DECODE: &'static str = "base64_decode";
    pub const HEX_DECODE: &'static str = "hex_decode";

    pub fn normalize(stepType: &str) -> Result<String, String> {
        let trimmed = stepType.trim();
        let normalized = trimmed.to_ascii_lowercase();
        match normalized.as_str() {
            Self::PARSE_JSON
            | Self::PICK
            | Self::PARSE_JSON_STRING
            | Self::HTTP_GET
            | Self::HTTP_REQUEST_FROM_OBJECT
            | Self::BASE64_DECODE
            | Self::HEX_DECODE => Ok(normalized),
            _ => Err(format!(
                "unsupported http tts response pipeline step: {trimmed}"
            )),
        }
    }
}
