use jni::objects::{JObject, JString, JValue};
use operit_host_api::{
    HostError, HostResult, LocalInferenceHost, LocalSttInferenceHostRequest,
    LocalSttInferenceHostResponse, LocalTtsInferenceHostRequest, LocalTtsInferenceHostResponse,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::secret_store::androidHostSecretStoreBridge;

#[derive(Clone, Default)]
pub struct AndroidLocalInferenceHost;

impl AndroidLocalInferenceHost {
    /// Creates an Android local inference host backed by the registered Java runtime host.
    pub fn new() -> Self {
        Self
    }
}

impl LocalInferenceHost for AndroidLocalInferenceHost {
    /// Transcribes one local audio request through the Android Sherpa ONNX host.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse> {
        callLocalInferenceHost("transcribeLocalSpeech", &request)
    }

    /// Synthesizes one local speech request through the Android Sherpa ONNX host.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse> {
        callLocalInferenceHost("synthesizeLocalSpeech", &request)
    }
}

/// Calls one JSON local inference method on the registered Java runtime host.
#[allow(non_snake_case)]
fn callLocalInferenceHost<Request, Response>(
    methodName: &str,
    request: &Request,
) -> HostResult<Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let bridge = androidHostSecretStoreBridge()?;
    let requestJson = serde_json::to_string(request).map_err(|error| {
        HostError::new(format!(
            "Android local inference request serialization failed: {error}"
        ))
    })?;
    let mut env = bridge.vm.attach_current_thread().map_err(|error| {
        HostError::new(format!(
            "Android local inference thread attachment failed: {error}"
        ))
    })?;
    let requestString = env.new_string(requestJson).map_err(|error| {
        HostError::new(format!(
            "Android local inference request allocation failed: {error}"
        ))
    })?;
    let requestObject = JObject::from(requestString);
    let responseObject = env
        .call_method(
            bridge.host.as_obj(),
            methodName,
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&requestObject)],
        )
        .map_err(|error| {
            HostError::new(format!(
                "Android local inference method {methodName} failed: {error}"
            ))
        })?
        .l()
        .map_err(|error| {
            HostError::new(format!(
                "Android local inference method {methodName} returned an invalid object: {error}"
            ))
        })?;
    if responseObject.is_null() {
        return Err(HostError::new(format!(
            "Android local inference method {methodName} returned null"
        )));
    }
    let responseString = JString::from(responseObject);
    let responseJson: String = env
        .get_string(&responseString)
        .map_err(|error| {
            HostError::new(format!(
                "Android local inference response decoding failed: {error}"
            ))
        })?
        .into();
    serde_json::from_str::<Response>(&responseJson).map_err(|error| {
        HostError::new(format!(
            "Android local inference response JSON is invalid: {error}"
        ))
    })
}
