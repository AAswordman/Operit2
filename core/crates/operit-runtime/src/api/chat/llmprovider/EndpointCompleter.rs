use crate::data::model::ModelConfigData::ApiProviderType;

pub struct EndpointCompleter;

impl EndpointCompleter {
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
                    if pathWithoutSlash.to_ascii_lowercase().ends_with("/anthropic") {
                        return format!("{endpointWithoutSlash}/v1/messages");
                    }
                    if pathWithoutSlash.to_ascii_lowercase().ends_with("/v1") {
                        return format!("{endpointWithoutSlash}/messages");
                    }
                }
                endpoint.to_string()
            }
            ApiProviderType::GOOGLE | ApiProviderType::GEMINI_GENERIC | ApiProviderType::MNN => {
                endpoint.to_string()
            }
            _ => Self::completeEndpoint(endpoint),
        }
    }
}

fn parse_path(endpoint: &str) -> Option<String> {
    let scheme_pos = endpoint.find("://")?;
    let after_host = &endpoint[scheme_pos + 3..];
    let slash_pos = after_host.find('/');
    match slash_pos {
        Some(index) => Some(after_host[index..].to_string()),
        None => Some(String::new()),
    }
}
