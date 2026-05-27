use operit_host_api::{HostResult, HttpHost, HttpRequestData, HttpResponseData};

use crate::common::{call_http, http_request_to_js, js_http_response};

#[derive(Clone, Debug, Default)]
pub struct WebHttpHost;

impl WebHttpHost {
    pub fn new() -> Self {
        Self
    }
}

impl HttpHost for WebHttpHost {
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        js_http_response(call_http("executeHttpRequest", &[http_request_to_js(request)])?)
    }
}
