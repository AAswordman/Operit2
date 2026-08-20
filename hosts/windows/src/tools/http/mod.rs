use operit_host_api::{
    HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpRequestData, HttpResponseData, HttpStreamChunkCallback,
    HttpStreamClosedCallback, HttpStreamHost, HttpStreamOpenedCallback,
    WebSocketClosedCallback, WebSocketHost, WebSocketMessageCallback, WebSocketOpenedCallback,
    WebSocketRequestData,
};
use operit_host_native_common::NativeHttpHost;

#[derive(Clone, Debug, Default)]
pub struct WindowsHttpHost {
    inner: NativeHttpHost,
}

impl HttpStreamHost for WindowsHttpHost {
    /// Opens one Windows HTTP byte stream through the shared native Host implementation.
    #[allow(non_snake_case)]
    fn openHttpByteStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onOpened: HttpStreamOpenedCallback,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.inner
            .openHttpByteStream(streamId, request, onOpened, onChunk, onClosed)
    }

    /// Closes one Windows HTTP byte stream.
    #[allow(non_snake_case)]
    fn closeHttpByteStream(&self, streamId: &str) -> HostResult<()> {
        self.inner.closeHttpByteStream(streamId)
    }
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

impl WebSocketHost for WindowsHttpHost {
    /// Opens one Windows WebSocket through the shared native Host implementation.
    #[allow(non_snake_case)]
    fn openWebSocket(
        &self,
        streamId: String,
        request: WebSocketRequestData,
        onOpened: WebSocketOpenedCallback,
        onMessage: WebSocketMessageCallback,
        onClosed: WebSocketClosedCallback,
    ) -> HostResult<()> {
        self.inner
            .openWebSocket(streamId, request, onOpened, onMessage, onClosed)
    }

    /// Sends one binary message through a Windows WebSocket.
    #[allow(non_snake_case)]
    fn sendWebSocketMessage(&self, streamId: &str, message: Vec<u8>) -> HostResult<()> {
        self.inner.sendWebSocketMessage(streamId, message)
    }

    /// Closes one Windows WebSocket.
    #[allow(non_snake_case)]
    fn closeWebSocket(&self, streamId: &str) -> HostResult<()> {
        self.inner.closeWebSocket(streamId)
    }
}
