use operit_host_api::{HostError, HostResult, WebVisitHost, WebVisitRequest, WebVisitResult};

#[derive(Clone, Debug, Default)]
pub struct AndroidWebVisitHost;

impl AndroidWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl WebVisitHost for AndroidWebVisitHost {
    fn visitWeb(&self, _request: WebVisitRequest) -> HostResult<WebVisitResult> {
        Err(HostError::new(
            "Android visit_web requires the Android WebView host bridge",
        ))
    }
}
