use std::sync::Arc;

use operit_host_api::{
    HostError, HostResult, LocalInferenceHost, LocalSttInferenceHostRequest,
    LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest, LocalTtsInferenceHostResponse,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub type AppleLocalInferenceExecutor =
    Arc<dyn Fn(AppleLocalInferenceCommand) -> HostResult<String> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct AppleLocalInferenceCommand {
    pub method: String,
    pub requestJson: String,
}

#[derive(Clone)]
pub struct AppleLocalInferenceHost {
    executor: AppleLocalInferenceExecutor,
}

impl AppleLocalInferenceHost {
    /// Creates an Apple local inference host from an owner-host executor.
    pub fn fromExecutor(executor: AppleLocalInferenceExecutor) -> Self {
        Self { executor }
    }
}

impl LocalInferenceHost for AppleLocalInferenceHost {
    /// Transcribes one local audio request through the Apple owner host.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        callExecutor(&self.executor, "transcribeLocalSpeech", &request)
    }

    /// Synthesizes one local speech request through the Apple owner host.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        callExecutor(&self.executor, "synthesizeLocalSpeech", &request)
    }
}

/// Sends one serialized local inference request to the Apple owner host.
fn callExecutor<Request, Response>(
    executor: &AppleLocalInferenceExecutor,
    method: &str,
    request: &Request,
) -> HostResult<Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let requestJson = serde_json::to_string(request).map_err(|error| {
        HostError::new(format!(
            "Apple local inference request JSON encode failed: {error}"
        ))
    })?;
    let resultJson = executor(AppleLocalInferenceCommand {
        method: method.to_string(),
        requestJson,
    })?;
    serde_json::from_str::<Response>(&resultJson).map_err(|error| {
        HostError::new(format!(
            "Apple local inference response JSON decode failed: {error}"
        ))
    })
}
