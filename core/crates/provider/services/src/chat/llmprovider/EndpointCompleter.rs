use operit_model::ModelConfigData::ApiProviderType;
use url::Url;

pub struct EndpointCompleter;

impl EndpointCompleter {
    /// Completes an OpenAI-compatible chat endpoint when the user supplied a host or version path.
    pub fn completeEndpoint(endpoint: &str) -> String {
        let trimmedEndpoint = endpoint.trim();
        if trimmedEndpoint.ends_with('#') {
            return trimmedEndpoint.trim_end_matches('#').to_string();
        }

        let endpointWithoutSlash = trimmedEndpoint.trim_end_matches('/');
        let path = parse_path(trimmedEndpoint);

        if let Some(path) = path {
            let pathWithoutSlash = path.trim_end_matches('/');
            if pathWithoutSlash.is_empty() {
                return format!("{endpointWithoutSlash}/v1/chat/completions");
            }
            if pathWithoutSlash.to_ascii_lowercase().ends_with("/v1") {
                return format!("{endpointWithoutSlash}/chat/completions");
            }
        }

        endpoint.to_string()
    }

    /// Completes an OpenAI Responses endpoint when the user supplied a host or version path.
    pub fn completeResponsesEndpoint(endpoint: &str) -> String {
        let trimmedEndpoint = endpoint.trim();
        if trimmedEndpoint.ends_with('#') {
            return trimmedEndpoint.trim_end_matches('#').to_string();
        }

        let endpointWithoutSlash = trimmedEndpoint.trim_end_matches('/');
        let path = parse_path(trimmedEndpoint);

        if let Some(path) = path {
            let pathWithoutSlash = path.trim_end_matches('/');
            if pathWithoutSlash.is_empty() {
                return format!("{endpointWithoutSlash}/v1/responses");
            }
            if pathWithoutSlash.to_ascii_lowercase().ends_with("/v1") {
                return format!("{endpointWithoutSlash}/responses");
            }
        }

        endpoint.to_string()
    }

    /// Completes the endpoint according to the selected provider protocol.
    pub fn completeEndpointForProviderType(
        endpoint: &str,
        providerType: ApiProviderType,
    ) -> String {
        let trimmedEndpoint = endpoint.trim();
        if trimmedEndpoint.ends_with('#') {
            return trimmedEndpoint.trim_end_matches('#').to_string();
        }

        let endpointWithoutSlash = trimmedEndpoint.trim_end_matches('/');
        match providerType {
            ApiProviderType::OPENAI_RESPONSES | ApiProviderType::OPENAI_RESPONSES_GENERIC => {
                Self::completeResponsesEndpoint(endpoint)
            }
            ApiProviderType::ANTHROPIC | ApiProviderType::ANTHROPIC_GENERIC => {
                if let Some(path) = parse_path(trimmedEndpoint) {
                    let pathWithoutSlash = path.trim_end_matches('/');
                    if pathWithoutSlash.is_empty() {
                        return format!("{endpointWithoutSlash}/v1/messages");
                    }
                    if pathWithoutSlash
                        .to_ascii_lowercase()
                        .ends_with("/anthropic")
                    {
                        return format!("{endpointWithoutSlash}/v1/messages");
                    }
                    if pathWithoutSlash.to_ascii_lowercase().ends_with("/v1") {
                        return format!("{endpointWithoutSlash}/messages");
                    }
                }
                endpoint.to_string()
            }
            ApiProviderType::GOOGLE
            | ApiProviderType::GEMINI_GENERIC
            | ApiProviderType::LOCAL_MODEL => endpoint.to_string(),
            _ => Self::completeEndpoint(endpoint),
        }
    }
}

/// Parses the URL path using the same path semantics as the Kotlin implementation.
fn parse_path(endpoint: &str) -> Option<String> {
    Url::parse(endpoint).ok().map(|url| {
        let path = url.path();
        if path == "/" {
            String::new()
        } else {
            path.to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps an already complete SenseNova chat endpoint unchanged.
    #[test]
    fn complete_endpoint_keeps_sensenova_chat_url() {
        assert_eq!(
            EndpointCompleter::completeEndpoint("https://token.sensenova.cn/v1/chat/completions"),
            "https://token.sensenova.cn/v1/chat/completions"
        );
    }

    /// Expands a SenseNova host to the chat completions endpoint.
    #[test]
    fn complete_endpoint_expands_sensenova_host() {
        assert_eq!(
            EndpointCompleter::completeEndpoint("https://token.sensenova.cn"),
            "https://token.sensenova.cn/v1/chat/completions"
        );
    }

    /// Expands a versioned SenseNova endpoint to the chat completions endpoint.
    #[test]
    fn complete_endpoint_expands_sensenova_v1_path() {
        assert_eq!(
            EndpointCompleter::completeEndpoint("https://token.sensenova.cn/v1"),
            "https://token.sensenova.cn/v1/chat/completions"
        );
    }

    /// Leaves Gemini endpoints unchanged because Gemini builds its own route.
    #[test]
    fn complete_endpoint_keeps_gemini_endpoint() {
        assert_eq!(
            EndpointCompleter::completeEndpointForProviderType(
                "https://generativelanguage.googleapis.com",
                ApiProviderType::GEMINI_GENERIC,
            ),
            "https://generativelanguage.googleapis.com"
        );
    }

    /// Expands Anthropic hosts to the messages endpoint.
    #[test]
    fn complete_endpoint_expands_anthropic_host() {
        assert_eq!(
            EndpointCompleter::completeEndpointForProviderType(
                "https://api.anthropic.com",
                ApiProviderType::ANTHROPIC_GENERIC,
            ),
            "https://api.anthropic.com/v1/messages"
        );
    }

    /// Expands Responses hosts to the responses endpoint.
    #[test]
    fn complete_endpoint_expands_responses_host() {
        assert_eq!(
            EndpointCompleter::completeEndpointForProviderType(
                "https://api.openai.com",
                ApiProviderType::OPENAI_RESPONSES_GENERIC,
            ),
            "https://api.openai.com/v1/responses"
        );
    }
}
