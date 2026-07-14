use operit_host_api::{
    HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpRequestData, HttpResponseData,
};
use operit_host_native_common::NativeHttpHost;

#[derive(Clone, Debug, Default)]
pub struct WindowsHttpHost {
    inner: NativeHttpHost,
}

impl WindowsHttpHost {
    /// Creates the Windows HTTP host.
    pub fn new() -> Self {
        Self {
            inner: NativeHttpHost::new(),
        }
    }
}

impl HttpHost for WindowsHttpHost {
    /// Executes one buffered HTTP request through the native host implementation.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        self.inner.executeHttpRequest(request)
    }

    /// Downloads files through the native bounded worker pool.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        self.inner.downloadFiles(request, control, onProgress)
    }
}
