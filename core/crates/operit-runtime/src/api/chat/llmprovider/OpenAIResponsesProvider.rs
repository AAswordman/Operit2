use super::AIService::AIService;

pub struct OpenAIResponsesProvider;

pub struct OpenAIResponsesPayloadAdapter;

impl OpenAIResponsesProvider {
    pub fn create_request_body(&self) {}

    pub fn customize_final_request_object(&self) {}

    pub fn apply_responses_reasoning_effort(&self) {}
}

impl OpenAIResponsesPayloadAdapter {
    pub fn map_parameter_name_for_responses(api_name: &str) -> String {
        api_name.to_string()
    }

    pub fn parse_usage_counts() {}

    pub fn to_responses_request() {}

    pub fn parse_non_streaming_response() {}
}

impl AIService for OpenAIResponsesProvider {}
