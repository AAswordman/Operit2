use operit_host_api::{
    HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpRequestData, HttpResponseData, HttpStreamChunkCallback,
    HttpStreamClosedCallback, HttpStreamHost, HttpStreamOpenedCallback, HttpStreamResponseCallback,
    WebSocketClosedCallback, WebSocketHost, WebSocketMessageCallback, WebSocketOpenedCallback,
    WebSocketRequestData,
};

#[derive(Clone, Debug, Default)]
pub struct AndroidHttpHost {
    inner: operit_host_native_common::NativeHttpHost,
}

impl HttpStreamHost for AndroidHttpHost {
    /// Streams response headers and body through the shared native network host.
    #[allow(non_snake_case)]
    fn openHttpResponseStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onResponse: HttpStreamResponseCallback,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.inner
            .openHttpResponseStream(streamId, request, onResponse, onChunk, onClosed)
    }

    /// Opens one Android HTTP byte stream through the shared native Host implementation.
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

    /// Closes one Android HTTP byte stream.
    #[allow(non_snake_case)]
    fn closeHttpByteStream(&self, streamId: &str) -> HostResult<()> {
        self.inner.closeHttpByteStream(streamId)
    }
}

impl AndroidHttpHost {
    /// Creates the Android HTTP host.
    pub fn new() -> Self {
        Self {
            inner: operit_host_native_common::NativeHttpHost::new(),
        }
    }
}

impl HttpHost for AndroidHttpHost {
    /// Preserves the underlying transport's response delivery capability.
    fn responseDelivery(&self) -> operit_host_api::HttpResponseDelivery {
        self.inner.responseDelivery()
    }

    /// Declares the image delivery supported by this HTTP host.
    fn imageDelivery(&self) -> operit_host_api::HttpImageDelivery {
        self.inner.imageDelivery()
    }

    /// Executes one buffered HTTP request through the native host implementation.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        self.inner.executeHttpRequest(request)
    }

    /// Streams one HTTP response directly into the destination file.
    #[allow(non_snake_case)]
    fn downloadToFile(
        &self,
        request: HttpRequestData,
        targetPath: String,
    ) -> HostResult<operit_host_api::HttpFileDownloadResult> {
        self.inner.downloadToFile(request, targetPath)
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

impl WebSocketHost for AndroidHttpHost {
    /// Opens one Android WebSocket through the shared native Host implementation.
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

    /// Sends one binary message through an Android WebSocket.
    #[allow(non_snake_case)]
    fn sendWebSocketMessage(&self, streamId: &str, message: Vec<u8>) -> HostResult<()> {
        self.inner.sendWebSocketMessage(streamId, message)
    }

    /// Closes one Android WebSocket.
    #[allow(non_snake_case)]
    fn closeWebSocket(&self, streamId: &str) -> HostResult<()> {
        self.inner.closeWebSocket(streamId)
    }
}
