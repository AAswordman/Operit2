use super::{ApiKeyPoolState, KeyGetter};
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpImageDelivery, HttpRequestData, HttpResponseData,
    HttpResponseHead, HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
    HttpStreamOpenedCallback, HttpStreamResponseCallback,
};
use operit_model::ApiKeyInfo::ApiKeyInfo;
use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct ScriptHost {
    replies: Mutex<VecDeque<(i32, bool)>>,
    pub requests: Mutex<Vec<HttpRequestData>>,
}

impl ScriptHost {
    /// Creates an isolated HTTP script with optional body disconnections.
    pub fn new(replies: &[(i32, bool)]) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.iter().copied().collect()),
            requests: Mutex::new(Vec::new()),
        })
    }
    /// Records actual transmitted credentials and consumes one scripted response.
    fn respond(&self, request: HttpRequestData) -> (i32, bool) {
        self.requests.lock().unwrap().push(request);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected HTTP attempt")
    }
    /// Reads one authentication header from each transmitted request.
    pub fn header_values(&self, name: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| {
                request
                    .headers
                    .iter()
                    .find(|(existing, _)| existing.eq_ignore_ascii_case(name))
                    .unwrap()
                    .1
                    .clone()
            })
            .collect()
    }

    /// Reads one authentication query parameter from each transmitted request.
    pub fn query_values(&self, name: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| {
                url::Url::parse(&request.url)
                    .unwrap()
                    .query_pairs()
                    .find(|(existing, _)| existing == name)
                    .unwrap()
                    .1
                    .into_owned()
            })
            .collect()
    }
}

impl HttpStreamHost for ScriptHost {
    /// Excludes the obsolete metadata-free transport.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("metadata required")
    }
    /// Delivers a complete response or a deliberate mid-body failure.
    fn openHttpResponseStream(
        &self,
        _: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let url = request.url.clone();
        let (status, broken) = self.respond(request);
        head(HttpResponseHead {
            finalUrl: url,
            statusCode: status,
            statusMessage: String::new(),
            headers: vec![("Retry-After".into(), "60".into())],
        });
        chunk(br#"{"content":[{"type":"text","text":"ok"}]}"#.to_vec());
        closed(if broken {
            Err("fixture disconnected".into())
        } else {
            Ok(())
        });
        Ok(())
    }
    /// Confirms cancellation without changing key health.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        Ok(())
    }
}

impl HttpHost for ScriptHost {
    /// Uses in-memory image delivery for the fixture host.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    /// Provides synchronous discovery responses through the same script.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let url = request.url.clone();
        let (status, broken) = self.respond(request);
        if broken {
            return Err(HostError::new("fixture disconnected"));
        }
        Ok(HttpResponseData {
            finalUrl: url,
            statusCode: status,
            statusMessage: String::new(),
            headers: Vec::new(),
            body: br#"{"data":[]}"#.to_vec(),
        })
    }
    /// Excludes downloads from the credential test.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("not used"))
    }
}

/// Connects the real policy to deterministic isolated feedback storage.
pub fn fixture() -> (KeyGetter, Arc<Mutex<(ProviderProfile, ApiKeyPoolState)>>) {
    let mut provider = ProviderProfile::new(
        "pool".into(),
        "Pool".into(),
        ApiProviderType::OPENAI_GENERIC,
        "https://fixture.invalid/v1".into(),
    );
    provider.useMultipleApiKeys = true;
    provider.apiKeyPool = (0..3)
        .map(|i| ApiKeyInfo::new(i.to_string(), format!("key-{i}")))
        .collect();
    let state = Arc::new(Mutex::new((provider, ApiKeyPoolState::default())));
    let acquire = state.clone();
    let report = state.clone();
    (
        KeyGetter::from_callbacks(
            Arc::new(move || {
                let mut state = acquire.lock().unwrap();
                let (provider, policy) = &mut *state;
                policy.reserve(provider, operit_host_api::TimeUtils::currentTimeMillis())
            }),
            Arc::new(move |key, started, outcome| {
                let mut state = report.lock().unwrap();
                let (provider, policy) = &mut *state;
                policy.report(
                    provider,
                    key,
                    started,
                    outcome,
                    operit_host_api::TimeUtils::currentTimeMillis(),
                );
                Ok(())
            }),
        ),
        state,
    )
}
