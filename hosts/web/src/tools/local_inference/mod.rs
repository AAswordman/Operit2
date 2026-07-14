use operit_host_api::{
    HostError, HostResult, LocalInferenceHost, LocalSttInferenceHostRequest,
    LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest, LocalTtsInferenceHostResponse,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::common::{call_local_inference, js_string};

#[derive(Clone, Debug, Default)]
pub struct WebLocalInferenceHost;

unsafe impl Send for WebLocalInferenceHost {}
unsafe impl Sync for WebLocalInferenceHost {}

impl WebLocalInferenceHost {
    /// Creates the browser local inference host.
    pub fn new() -> Self {
        Self
    }
}

impl LocalInferenceHost for WebLocalInferenceHost {
    /// Transcribes one local audio request through the browser inference host.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        callLocalInference("transcribeLocalSpeech", &request)
    }

    /// Synthesizes one local speech request through the browser inference host.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        callLocalInference("synthesizeLocalSpeech", &request)
    }
}

/// Calls one JSON local inference method on the browser host bridge.
fn callLocalInference<Request, Response>(method: &str, request: &Request) -> HostResult<Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let requestJson = serde_json::to_string(request).map_err(|error| {
        HostError::new(format!(
            "Web local inference request JSON encode failed: {error}"
        ))
    })?;
    let responseJson = js_string(
        call_local_inference(method, &[JsValue::from_str(&requestJson)])?,
        "web local inference",
    )?;
    serde_json::from_str::<Response>(&responseJson).map_err(|error| {
        HostError::new(format!(
            "Web local inference response JSON decode failed: {error}"
        ))
    })
}
