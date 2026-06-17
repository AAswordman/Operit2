use std::thread;

use operit_host_api::{HostError, HostResult, HttpHost, HttpRequestData, HttpResponseData};

#[derive(Clone, Debug, Default)]
pub struct AndroidHttpHost {
    inner: operit_host_linux_native::LinuxHttpHost,
}

impl AndroidHttpHost {
    pub fn new() -> Self {
        Self {
            inner: operit_host_linux_native::LinuxHttpHost::new(),
        }
    }
}

impl HttpHost for AndroidHttpHost {
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let inner = self.inner.clone();
        thread::spawn(move || inner.executeHttpRequest(request))
            .join()
            .map_err(|_| HostError::new("android HTTP request thread panicked"))?
    }
}
