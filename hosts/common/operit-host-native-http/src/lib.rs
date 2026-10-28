pub mod server;
mod connector;
use futures_util::{SinkExt, StreamExt};
use operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost;
pub use server::NativeHttpServerHost;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tokio_tungstenite::tungstenite;

use operit_host_api::{
    httpDownloadPartialTargetPath, HostError, HostResult, HttpDownloadControl,
    HttpDownloadFileRequest, HttpDownloadFileResult, HttpDownloadProgress,
    HttpDownloadProgressCallback, HttpDownloadProgressState, HttpDownloadRequest,
    HttpDownloadResult, HttpFileDownloadResult, HttpHost, HttpRequestData, HttpResponseData,
    HttpResponseHead, HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost,
    HttpStreamOpenedCallback, HttpStreamResponseCallback, WebSocketClosedCallback, WebSocketHost,
    WebSocketMessageCallback, WebSocketOpenedCallback, WebSocketRequestData,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_RANGE, RANGE};
use reqwest::multipart;
use reqwest::{Client as AsyncClient, Method, Proxy, StatusCode};
use tungstenite::client::IntoClientRequest;

#[derive(Clone, Debug, Default)]
pub struct NativeHttpHost {
    byteStreams: Arc<Mutex<BTreeMap<String, tokio::sync::watch::Sender<bool>>>>,
    webSockets: Arc<Mutex<BTreeMap<String, NativeWebSocketRegistration>>>,
    httpClients: Arc<Mutex<BTreeMap<HttpClientPolicy, AsyncClient>>>,
}

/// Identifies the connection pool for an exact buffered HTTP request policy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HttpClientPolicy {
    connectTimeoutSeconds: u64,
    readTimeoutSeconds: u64,
    followRedirects: bool,
    ignoreSsl: bool,
    proxyHost: String,
    proxyPort: u16,
}

/// Owns cancellation independently of a bounded WebSocket send queue.
#[derive(Clone, Debug)]
struct NativeWebSocketRegistration {
    commands: tokio::sync::mpsc::Sender<tungstenite::Message>,
    cancel: tokio::sync::watch::Sender<bool>,
}

impl NativeHttpHost {
    /// Creates a native HTTP host.
    pub fn new() -> Self {
        Self::default()
    }
}

impl NativeHttpHost {
    /// Opens one socket on the Host's shared four-worker network executor.
    #[allow(non_snake_case)]
    fn openNativeWebSocket(
        &self,
        streamId: String,
        request: WebSocketRequestData,
        onOpened: WebSocketOpenedCallback,
        onMessage: WebSocketMessageCallback,
        onClosed: WebSocketClosedCallback,
    ) -> HostResult<()> {
        let (commands, receiver) = tokio::sync::mpsc::channel(64);
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        let registration = NativeWebSocketRegistration { commands, cancel };
        {
            let mut sockets = self
                .webSockets
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            if sockets.contains_key(&streamId) {
                return Err(HostError::new(format!(
                    "WebSocket is already open: {streamId}"
                )));
            }
            sockets.insert(streamId.clone(), registration.clone());
        }
        let sockets = self.webSockets.clone();
        let task_id = streamId.clone();
        let result = NativeHostRuntimeTaskSchedulerHost::new().scheduleNetworkAsyncTask(
            "native-websocket",
            Box::new(move || {
                Box::pin(async move {
                    let result =
                        executeNativeWebSocket(request, receiver, cancelled, onOpened, onMessage)
                            .await
                            .map_err(|error| error.to_string());
                    {
                        let mut sockets = sockets.lock().expect("WebSocket lock poisoned");
                        if sockets
                            .get(&task_id)
                            .is_some_and(|entry| entry.cancel.same_channel(&registration.cancel))
                        {
                            sockets.remove(&task_id);
                        }
                    }
                    onClosed(result);
                })
            }),
        );
        if result.is_err() {
            self.webSockets
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(&streamId);
        }
        result
    }
}

impl WebSocketHost for NativeHttpHost {
    /// Opens one native binary WebSocket and forwards its lifecycle callbacks.
    #[allow(non_snake_case)]
    fn openWebSocket(
        &self,
        streamId: String,
        request: WebSocketRequestData,
        onOpened: WebSocketOpenedCallback,
        onMessage: WebSocketMessageCallback,
        onClosed: WebSocketClosedCallback,
    ) -> HostResult<()> {
        self.openNativeWebSocket(streamId, request, onOpened, onMessage, onClosed)
    }

    /// Sends one binary message through a native WebSocket.
    #[allow(non_snake_case)]
    fn sendWebSocketMessage(&self, streamId: &str, message: Vec<u8>) -> HostResult<()> {
        let sender = self
            .webSockets
            .lock()
            .map_err(|error| HostError::new(format!("WebSocket lock poisoned: {error}")))?
            .get(streamId)
            .cloned()
            .ok_or_else(|| HostError::new(format!("WebSocket is not open: {streamId}")))?;
        sender
            .commands
            .try_send(tungstenite::Message::Binary(message))
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Sends one UTF-8 text frame through a native WebSocket.
    #[allow(non_snake_case)]
    fn sendWebSocketText(&self, streamId: &str, text: String) -> HostResult<()> {
        let sender = self.webSockets.lock().map_err(|error| HostError::new(error.to_string()))?
            .get(streamId).cloned().ok_or_else(|| HostError::new(format!("WebSocket is not open: {streamId}")))?;
        sender.commands.try_send(tungstenite::Message::Text(text)).map_err(|error| HostError::new(error.to_string()))
    }

    /// Closes one native WebSocket.
    #[allow(non_snake_case)]
    fn closeWebSocket(&self, streamId: &str) -> HostResult<()> {
        let sender = self
            .webSockets
            .lock()
            .map_err(|error| HostError::new(format!("WebSocket lock poisoned: {error}")))?
            .remove(streamId)
            .ok_or_else(|| HostError::new(format!("WebSocket is not open: {streamId}")))?;
        sender.cancel.send_replace(true);
        Ok(())
    }
}

/// Drives a socket and its command queue without blocking a network worker.
async fn executeNativeWebSocket(
    request: WebSocketRequestData,
    mut commands: tokio::sync::mpsc::Receiver<tungstenite::Message>,
    mut cancelled: tokio::sync::watch::Receiver<bool>,
    onOpened: WebSocketOpenedCallback,
    onMessage: WebSocketMessageCallback,
) -> HostResult<()> {
    let mut handshake = request
        .url
        .into_client_request()
        .map_err(|error| HostError::new(error.to_string()))?;
    for (name, value) in request.headers {
        handshake.headers_mut().insert(
            tungstenite::http::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| HostError::new(error.to_string()))?,
            tungstenite::http::header::HeaderValue::from_str(&value)
                .map_err(|error| HostError::new(error.to_string()))?,
        );
    }
    let connect = async {
        let handshake = tokio_tungstenite::connect_async(handshake);
        if request.connectTimeoutSeconds == 0 {
            handshake
                .await
                .map_err(|error| HostError::new(error.to_string()))
        } else {
            tokio::time::timeout(
                Duration::from_secs(request.connectTimeoutSeconds),
                handshake,
            )
            .await
            .map_err(|_| HostError::timeout("WebSocket connect deadline exceeded"))?
            .map_err(|error| HostError::new(error.to_string()))
        }
    };
    let (mut socket, _) = tokio::select! {
        biased;
        _ = cancelled.changed() => return Ok(()),
        connected = connect => connected?,
    };
    onOpened();
    loop {
        tokio::select! {
            biased;
            _ = cancelled.changed() => { let _ = socket.close(None).await; return Ok(()); }
            command = commands.recv() => match command {
                Some(message) => socket.send(message).await.map_err(|error| HostError::new(error.to_string()))?,
                None => return Ok(()),
            },
            message = socket.next() => match message {
                Some(Ok(tungstenite::Message::Binary(message))) => onMessage(message),
                Some(Ok(tungstenite::Message::Text(message))) => onMessage(message.into_bytes()),
                Some(Ok(tungstenite::Message::Close(Some(frame)))) => {
                    if frame.code != tungstenite::protocol::frame::coding::CloseCode::Normal {
                        return Err(HostError::new(format!("WebSocket closed with code {}: {}", u16::from(frame.code), frame.reason)));
                    }
                    return Ok(());
                }
                Some(Ok(tungstenite::Message::Close(None))) | None => return Ok(()),
                Some(Ok(_)) => {},
                Some(Err(error)) => return Err(HostError::new(error.to_string())),
            }
        }
    }
}

impl NativeHttpHost {
    /// Owns cancellation and thread cleanup for both metadata and byte-only streams.
    #[allow(non_snake_case)]
    fn openNativeHttpStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onOpened: HttpStreamOpenedCallback,
        onResponse: Option<HttpStreamResponseCallback>,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let (cancelSender, cancelReceiver) = tokio::sync::watch::channel(false);
        {
            let mut streams = self.byteStreams.lock().map_err(|error| {
                HostError::new(format!("HTTP byte stream lock poisoned: {error}"))
            })?;
            if streams.contains_key(&streamId) {
                return Err(HostError::new(format!(
                    "HTTP byte stream is already open: {streamId}"
                )));
            }
            streams.insert(streamId.clone(), cancelSender.clone());
        }
        let streams = self.byteStreams.clone();
        let taskStreamId = streamId.clone();
        let spawnResult = NativeHostRuntimeTaskSchedulerHost::new().scheduleNetworkAsyncTask(
            "native-http-stream",
            Box::new(move || {
                Box::pin(async move {
                    let result = executeHttpByteStream(
                        request,
                        cancelReceiver,
                        onOpened,
                        onResponse,
                        onChunk,
                    )
                    .await
                    .map_err(|error| error.to_string());
                    {
                        let mut streams = streams.lock().expect("HTTP byte stream lock poisoned");
                        if streams
                            .get(&taskStreamId)
                            .is_some_and(|entry| entry.same_channel(&cancelSender))
                        {
                            streams.remove(&taskStreamId);
                        }
                    }
                    onClosed(result);
                })
            }),
        );
        if let Err(error) = spawnResult {
            self.byteStreams
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(&streamId);
            return Err(error);
        }
        Ok(())
    }
}

impl HttpStreamHost for NativeHttpHost {
    /// Opens one byte-only stream without changing its existing status handling.
    #[allow(non_snake_case)]
    fn openHttpByteStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onOpened: HttpStreamOpenedCallback,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.openNativeHttpStream(streamId, request, onOpened, None, onChunk, onClosed)
    }

    /// Delivers response metadata before body chunks, including HTTP error responses.
    #[allow(non_snake_case)]
    fn openHttpResponseStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onResponse: HttpStreamResponseCallback,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.openNativeHttpStream(
            streamId,
            request,
            Arc::new(|| {}),
            Some(onResponse),
            onChunk,
            onClosed,
        )
    }

    /// Cancels one native asynchronous HTTP response stream.
    #[allow(non_snake_case)]
    fn closeHttpByteStream(&self, streamId: &str) -> HostResult<()> {
        let sender = self
            .byteStreams
            .lock()
            .map_err(|error| HostError::new(format!("HTTP byte stream lock poisoned: {error}")))?
            .remove(streamId)
            .ok_or_else(|| HostError::new(format!("HTTP byte stream is not open: {streamId}")))?;
        sender
            .send(true)
            .map_err(|error| HostError::new(error.to_string()))
    }
}

/// Runs one HTTP response stream until the server ends it or the Host receives cancellation.
#[allow(non_snake_case)]
async fn executeHttpByteStream(
    request: HttpRequestData,
    mut cancelReceiver: tokio::sync::watch::Receiver<bool>,
    onOpened: HttpStreamOpenedCallback,
    onResponse: Option<HttpStreamResponseCallback>,
    onChunk: HttpStreamChunkCallback,
) -> HostResult<()> {
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|error| HostError::new(error.to_string()))?;
    let client = buildHttpStreamClient(
        request.connectTimeoutSeconds,
        request.readTimeoutSeconds,
        request.followRedirects,
        request.ignoreSsl,
        &request.proxyHost,
        request.proxyPort,
    )?;
    let mut httpRequest = client.request(method, request.url);
    httpRequest = httpRequest.headers(headersToReqwest(&request.headers)?);
    if !request.fileParts.is_empty() || !request.formFields.is_empty() {
        let mut form = reqwest::multipart::Form::new();
        for (name, value) in request.formFields {
            form = form.text(name, value);
        }
        for file in request.fileParts {
            let part = reqwest::multipart::Part::bytes(file.content)
                .file_name(file.fileName)
                .mime_str(&file.contentType)
                .map_err(httpError)?;
            form = form.part(file.fieldName, part);
        }
        httpRequest = httpRequest.multipart(form);
    } else if !request.body.is_empty() {
        httpRequest = httpRequest.body(request.body);
    }
    let mut response = tokio::select! {
        changed = cancelReceiver.changed() => {
            changed.map_err(|error| HostError::new(error.to_string()))?;
            return Ok(());
        }
        response = sendOnNetworkTask(httpRequest) => {
            response?
        }
    };
    if let Some(onResponse) = onResponse {
        onResponse(HttpResponseHead {
            finalUrl: response.url().to_string(),
            statusCode: i32::from(response.status().as_u16()),
            statusMessage: response
                .status()
                .canonical_reason()
                .unwrap_or_default()
                .to_string(),
            headers: response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect(),
        });
    } else if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.map_err(httpError)?;
        return Err(HostError::new(format!("HTTP {status}: {body}")));
    }
    onOpened();
    loop {
        tokio::select! {
            changed = cancelReceiver.changed() => {
                changed.map_err(|error| HostError::new(error.to_string()))?;
                return Ok(());
            }
            chunk = response.chunk() => {
                match chunk.map_err(httpError)? {
                    Some(bytes) => onChunk(bytes.to_vec()),
                    None => return Ok(()),
                }
            }
        }
    }
}

impl HttpHost for NativeHttpHost {
    /// Delivers image bytes through cancellable HTTP streams.
    fn imageDelivery(&self) -> operit_host_api::HttpImageDelivery {
        operit_host_api::HttpImageDelivery::Bytes
    }

    /// Executes buffered HTTP through the shared asynchronous network executor.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let clients = self.httpClients.clone();
        NativeHostRuntimeTaskSchedulerHost::new().runNetworkTask("native-http-request", move || {
            executeHttpRequestAsync(request, clients)
        })
    }

    /// Streams a response to disk through the same Host network executor.
    fn downloadToFile(
        &self,
        request: HttpRequestData,
        targetPath: String,
    ) -> HostResult<HttpFileDownloadResult> {
        NativeHostRuntimeTaskSchedulerHost::new().runNetworkTask("native-http-file", move || {
            downloadToFileAsync(request, targetPath)
        })
    }

    /// Polls a bounded set of file futures without creating per-batch worker threads.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        NativeHostRuntimeTaskSchedulerHost::new()
            .runNetworkTask("native-http-downloads", move || {
                executeDownloadBatch(request, control, onProgress)
            })
    }
}

/// Streams one successful response directly to a temporary file and publishes
/// it only after the complete response has been written.
async fn downloadToFileAsync(
    request: HttpRequestData,
    targetPath: String,
) -> HostResult<HttpFileDownloadResult> {
    if targetPath.trim().is_empty() {
        return Err(HostError::new("HTTP download target path is empty"));
    }
    let client = buildHttpStreamClient(
        request.connectTimeoutSeconds,
        request.readTimeoutSeconds,
        request.followRedirects,
        request.ignoreSsl,
        &request.proxyHost,
        request.proxyPort,
    )?;
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut httpRequest = client.request(method, request.url);
    httpRequest = httpRequest.headers(headersToReqwest(&request.headers)?);
    if !request.fileParts.is_empty() || !request.formFields.is_empty() {
        let mut form = reqwest::multipart::Form::new();
        for (name, value) in request.formFields {
            form = form.text(name, value);
        }
        for file in request.fileParts {
            let part = reqwest::multipart::Part::bytes(file.content)
                .file_name(file.fileName)
                .mime_str(&file.contentType)
                .map_err(httpError)?;
            form = form.part(file.fieldName, part);
        }
        httpRequest = httpRequest.multipart(form);
    } else if !request.body.is_empty() {
        httpRequest = httpRequest.body(request.body);
    }
    let mut response = sendOnNetworkTask(httpRequest).await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.map_err(httpError)?;
        return Err(HostError::new(format!("HTTP {status}: {body}")));
    }
    let finalUrl = response.url().to_string();
    let target = Path::new(&targetPath);
    if let Some(parent) = target.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(httpIoError)?;
    }
    let partialPath = httpDownloadPartialTargetPath(&targetPath);
    let result = async {
        let mut output = fs::File::create(&partialPath).map_err(httpIoError)?;
        let mut downloadedBytes = 0u64;
        while let Some(chunk) = response.chunk().await.map_err(httpError)? {
            output.write_all(&chunk).map_err(httpIoError)?;
            downloadedBytes = downloadedBytes
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| HostError::new("HTTP download byte count overflowed"))?;
        }
        output.flush().map_err(httpIoError)?;
        drop(output);
        fs::rename(&partialPath, target).map_err(httpIoError)?;
        Ok(HttpFileDownloadResult {
            finalUrl,
            targetPath: targetPath.clone(),
            downloadedBytes,
        })
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(&partialPath);
    }
    result
}

/// Executes a buffered request on its Host-owned asynchronous network context.
async fn executeHttpRequestAsync(
    request: HttpRequestData,
    clients: Arc<Mutex<BTreeMap<HttpClientPolicy, AsyncClient>>>,
) -> HostResult<HttpResponseData> {
    let response = sendHttpRequestAsync(request, clients).await?;
    let finalUrl = response.url().to_string();
    let status = response.status();
    let statusCode = status.as_u16() as i32;
    let statusMessage = match status.canonical_reason() {
        Some(reason) => reason.to_string(),
        None => String::new(),
    };
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            value
                .to_str()
                .map(|text| (name.to_string(), text.to_string()))
                .map_err(|error| HostError::new(error.to_string()))
        })
        .collect::<HostResult<Vec<_>>>()?;
    let body = response.bytes().await.map_err(httpError)?.to_vec();
    Ok(HttpResponseData {
        finalUrl,
        statusCode,
        statusMessage,
        headers,
        body,
    })
}

/// Sends buffered and file requests with the same headers, body, proxy and redirect policy.
async fn sendHttpRequestAsync(
    request: HttpRequestData,
    clients: Arc<Mutex<BTreeMap<HttpClientPolicy, AsyncClient>>>,
) -> HostResult<reqwest::Response> {
    let policy = HttpClientPolicy {
        connectTimeoutSeconds: request.connectTimeoutSeconds,
        readTimeoutSeconds: request.readTimeoutSeconds,
        followRedirects: request.followRedirects,
        ignoreSsl: request.ignoreSsl,
        proxyHost: request.proxyHost.clone(),
        proxyPort: request.proxyPort,
    };
    let client = {
        let mut clients = clients
            .lock()
            .map_err(|error| HostError::new(format!("HTTP client pool lock poisoned: {error}")))?;
        if let Some(client) = clients.get(&policy) {
            client.clone()
        } else {
            let client = buildHttpStreamClient(
                policy.connectTimeoutSeconds,
                policy.readTimeoutSeconds,
                policy.followRedirects,
                policy.ignoreSsl,
                &policy.proxyHost,
                policy.proxyPort,
            )?;
            clients.insert(policy, client.clone());
            client
        }
    };
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut httpRequest = client.request(method, request.url);
    httpRequest = httpRequest.headers(headersToReqwest(&request.headers)?);
    if !request.fileParts.is_empty() || !request.formFields.is_empty() {
        let mut form = multipart::Form::new();
        for (name, value) in request.formFields {
            form = form.text(name, value);
        }
        for file in request.fileParts {
            let part = multipart::Part::bytes(file.content)
                .file_name(file.fileName)
                .mime_str(&file.contentType)
                .map_err(httpError)?;
            form = form.part(file.fieldName, part);
        }
        httpRequest = httpRequest.multipart(form);
    } else if !request.body.is_empty() {
        httpRequest = httpRequest.body(request.body);
    }
    sendOnNetworkTask(httpRequest).await
}

/// Isolates request construction and HTTP handshaking from the caller's poll frame.
/// Connection setup already uses the same cancellable task boundary; handshaking
/// happens after that connector returns and must not re-enter the larger caller.
async fn sendOnNetworkTask(request: reqwest::RequestBuilder) -> HostResult<reqwest::Response> {
    connector::runTask(async move { request.send().await })
        .await
        .map_err(|error| HostError::new(error.to_string()))?
        .map_err(httpError)
}

/// Preserves the network cause and timeout classification across the Host boundary.
fn httpError(error: reqwest::Error) -> HostError {
    let message = errorChain(&error);
    if error.is_timeout() {
        HostError::timeout(message)
    } else {
        HostError::new(message)
    }
}

/// Includes nested HTTP failures when streaming a response through std::io.
fn httpIoError(error: std::io::Error) -> HostError {
    let mut source: Option<&(dyn Error + 'static)> = Some(&error);
    let mut timedOut = error.kind() == std::io::ErrorKind::TimedOut;
    while let Some(cause) = source {
        if let Some(networkError) = cause.downcast_ref::<reqwest::Error>() {
            timedOut |= networkError.is_timeout();
        }
        source = cause.source();
    }
    let message = errorChain(&error);
    if timedOut {
        HostError::timeout(message)
    } else {
        HostError::new(message)
    }
}

fn errorChain(error: &(dyn Error + 'static)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Executes one validated batch through a bounded native worker pool.
async fn executeDownloadBatch(
    request: HttpDownloadRequest,
    control: HttpDownloadControl,
    onProgress: HttpDownloadProgressCallback,
) -> HostResult<HttpDownloadResult> {
    validateDownloadRequest(&request)?;
    let totalBytes = request.files.iter().try_fold(0u64, |total, file| {
        total
            .checked_add(file.expectedBytes)
            .ok_or_else(|| HostError::new("HTTP download declared byte total overflowed"))
    })?;
    let totalFiles = request.files.len();
    let workerCount = request.maxConcurrency.min(totalFiles);
    let client = buildDownloadHttpClient(
        request.connectTimeoutSeconds,
        request.readTimeoutSeconds,
        request.followRedirects,
        request.ignoreSsl,
        &request.proxyHost,
        request.proxyPort,
    )?;
    let results = Arc::new(Mutex::new(BTreeMap::<String, HttpDownloadFileResult>::new()));
    let failure = Arc::new(Mutex::new(None::<HostError>));
    let downloadedBytes = Arc::new(AtomicU64::new(0));
    let completedFiles = Arc::new(AtomicUsize::new(0));
    let progressGate = Arc::new(Mutex::new(()));

    let jobs = futures_util::stream::iter(request.files.iter().map(|file| async {
        if control.isCancelled() || downloadHasFailed(&failure) {
            return;
        }
        let result = downloadOneFileAsync(
            &client,
            &request.downloadId,
            file,
            totalBytes,
            totalFiles,
            &downloadedBytes,
            &completedFiles,
            &progressGate,
            &control,
            &failure,
            &onProgress,
        )
        .await;
        match result {
            Ok(result) => {
                results
                    .lock()
                    .expect("HTTP download results lock poisoned")
                    .insert(result.fileId.clone(), result);
            }
            Err(error) => recordDownloadFailure(&failure, error),
        }
    }))
    .buffer_unordered(workerCount);
    jobs.collect::<Vec<_>>().await;

    if control.isCancelled() {
        removeQueuedAndIncompleteTargets(&request.files, &results)?;
        return Err(HostError::new(format!(
            "HTTP download cancelled: {}",
            request.downloadId
        )));
    }
    if let Some(error) = takeDownloadFailure(&failure)? {
        removeQueuedAndIncompleteTargets(&request.files, &results)?;
        return Err(error);
    }
    let mut resultMap = results
        .lock()
        .map_err(|error| HostError::new(format!("HTTP download result lock poisoned: {error}")))?;
    let mut files = Vec::with_capacity(request.files.len());
    for file in &request.files {
        let result = resultMap.remove(&file.fileId).ok_or_else(|| {
            HostError::new(format!(
                "HTTP download result is missing for file: {}",
                file.fileId
            ))
        })?;
        files.push(result);
    }
    Ok(HttpDownloadResult {
        downloadId: request.downloadId,
        files,
        downloadedBytes: downloadedBytes.load(Ordering::SeqCst),
    })
}

/// Validates identifiers, targets, byte counts, and concurrency before worker creation.
fn validateDownloadRequest(request: &HttpDownloadRequest) -> HostResult<()> {
    if request.downloadId.trim().is_empty() {
        return Err(HostError::new("HTTP download id is empty"));
    }
    if request.files.is_empty() {
        return Err(HostError::new("HTTP download file list is empty"));
    }
    if request.maxConcurrency == 0 {
        return Err(HostError::new("HTTP download concurrency must be positive"));
    }
    let mut fileIds = BTreeSet::new();
    let mut targetPaths = BTreeSet::new();
    for file in &request.files {
        if file.fileId.trim().is_empty() {
            return Err(HostError::new("HTTP download file id is empty"));
        }
        if !fileIds.insert(file.fileId.clone()) {
            return Err(HostError::new(format!(
                "HTTP download file id is duplicated: {}",
                file.fileId
            )));
        }
        if file.url.trim().is_empty() {
            return Err(HostError::new(format!(
                "HTTP download URL is empty: {}",
                file.fileId
            )));
        }
        if file.targetPath.trim().is_empty() {
            return Err(HostError::new(format!(
                "HTTP download target path is empty: {}",
                file.fileId
            )));
        }
        if !targetPaths.insert(file.targetPath.clone()) {
            return Err(HostError::new(format!(
                "HTTP download target path is duplicated: {}",
                file.targetPath
            )));
        }
        if file.expectedBytes == 0 {
            return Err(HostError::new(format!(
                "HTTP download expected byte count is zero: {}",
                file.fileId
            )));
        }
    }
    Ok(())
}

/// Builds an asynchronous client for Host-owned streaming HTTP responses.
#[allow(non_snake_case)]
/// Builds a Host client whose connection setup has an independent cancellable poll stack.
fn buildHttpStreamClient(
    connectTimeoutSeconds: u64,
    readTimeoutSeconds: u64,
    followRedirects: bool,
    ignoreSsl: bool,
    proxyHost: &str,
    proxyPort: u16,
) -> HostResult<AsyncClient> {
    let mut builder = AsyncClient::builder()
        .connector_layer(connector::ConnectionTaskLayer)
        .danger_accept_invalid_certs(ignoreSsl);
    if connectTimeoutSeconds > 0 {
        builder = builder.connect_timeout(Duration::from_secs(connectTimeoutSeconds));
    }
    if readTimeoutSeconds > 0 {
        builder = builder.read_timeout(Duration::from_secs(readTimeoutSeconds));
    }
    if !followRedirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    if !proxyHost.trim().is_empty() {
        if proxyPort == 0 {
            return Err(HostError::new("HTTP proxy port must be positive"));
        }
        builder = builder.proxy(
            Proxy::all(format!("http://{}:{}", proxyHost.trim(), proxyPort)).map_err(httpError)?,
        );
    }
    builder.build().map_err(httpError)
}

fn buildDownloadHttpClient(
    connectTimeoutSeconds: u64,
    readTimeoutSeconds: u64,
    followRedirects: bool,
    ignoreSsl: bool,
    proxyHost: &str,
    proxyPort: u16,
) -> HostResult<AsyncClient> {
    buildHttpStreamClient(
        connectTimeoutSeconds,
        readTimeoutSeconds,
        followRedirects,
        ignoreSsl,
        proxyHost,
        proxyPort,
    )
}

/// Streams one HTTP file while racing every network wait against cancellation.
async fn downloadOneFileAsync(
    client: &AsyncClient,
    downloadId: &str,
    file: &HttpDownloadFileRequest,
    totalBytes: u64,
    totalFiles: usize,
    downloadedBytes: &AtomicU64,
    completedFiles: &AtomicUsize,
    progressGate: &Mutex<()>,
    control: &HttpDownloadControl,
    failure: &Mutex<Option<HostError>>,
    onProgress: &HttpDownloadProgressCallback,
) -> HostResult<HttpDownloadFileResult> {
    if control.isCancelled() {
        return Err(HostError::new(format!(
            "HTTP download cancelled: {downloadId}"
        )));
    }
    let target = Path::new(&file.targetPath);
    let parent = target.parent().ok_or_else(|| {
        HostError::new(format!(
            "HTTP download target parent is missing: {}",
            file.targetPath
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| HostError::new(error.to_string()))?;
    let partialPath = httpDownloadPartialTargetPath(&file.targetPath);
    let partialTarget = Path::new(&partialPath);
    let retainedBytes = prepareResumableDownload(target, partialTarget, file.expectedBytes)?;
    if retainedBytes == file.expectedBytes {
        completedFiles.fetch_add(1, Ordering::SeqCst);
        downloadedBytes.fetch_add(retainedBytes, Ordering::SeqCst);
        publishDownloadProgress(
            progressGate,
            downloadedBytes,
            completedFiles,
            onProgress,
            HttpDownloadProgress {
                downloadId: downloadId.to_string(),
                fileId: file.fileId.clone(),
                state: HttpDownloadProgressState::Completed,
                fileDownloadedBytes: retainedBytes,
                fileTotalBytes: file.expectedBytes,
                downloadedBytes: 0,
                totalBytes,
                completedFiles: 0,
                totalFiles,
            },
        )?;
        return Ok(HttpDownloadFileResult {
            fileId: file.fileId.clone(),
            finalUrl: file.url.clone(),
            targetPath: file.targetPath.clone(),
            downloadedBytes: retainedBytes,
        });
    }
    let mut request = client.get(&file.url);
    request = request.headers(headersToReqwest(&file.headers)?);
    if retainedBytes > 0 {
        request = request.header(RANGE, format!("bytes={retainedBytes}-"));
    }
    let mut response = tokio::select! {
        response = sendOnNetworkTask(request) => response?,
        () = waitForDownloadCancellation(control) => {
            return Err(HostError::new(format!("HTTP download cancelled: {downloadId}")));
        }
    };
    let expectedStatus = match retainedBytes {
        0 => StatusCode::OK,
        _ => StatusCode::PARTIAL_CONTENT,
    };
    if response.status() != expectedStatus {
        return Err(HostError::new(format!(
            "HTTP download request returned unexpected status: {} expected={} actual={}",
            file.url,
            expectedStatus,
            response.status()
        )));
    }
    if retainedBytes > 0 {
        validateContentRange(response.headers(), retainedBytes, file.expectedBytes)?;
    }
    let finalUrl = response.url().to_string();
    let mut output = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(partialTarget)
        .map_err(|error| HostError::new(error.to_string()))?;
    downloadedBytes.fetch_add(retainedBytes, Ordering::SeqCst);
    publishDownloadProgress(
        progressGate,
        downloadedBytes,
        completedFiles,
        onProgress,
        HttpDownloadProgress {
            downloadId: downloadId.to_string(),
            fileId: file.fileId.clone(),
            state: HttpDownloadProgressState::Started,
            fileDownloadedBytes: retainedBytes,
            fileTotalBytes: file.expectedBytes,
            downloadedBytes: 0,
            totalBytes,
            completedFiles: 0,
            totalFiles,
        },
    )?;
    let mut fileDownloadedBytes = retainedBytes;
    loop {
        if control.isCancelled() || downloadHasFailed(failure) {
            return Err(HostError::new(format!(
                "HTTP download interrupted: {downloadId}"
            )));
        }
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk.map_err(|error| HostError::new(error.to_string()))?,
            () = waitForDownloadCancellation(control) => {
                return Err(HostError::new(format!("HTTP download interrupted: {downloadId}")));
            }
        };
        let Some(chunk) = chunk else {
            break;
        };
        output
            .write_all(&chunk)
            .map_err(|error| HostError::new(error.to_string()))?;
        fileDownloadedBytes += chunk.len() as u64;
        downloadedBytes.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        publishDownloadProgress(
            progressGate,
            downloadedBytes,
            completedFiles,
            onProgress,
            HttpDownloadProgress {
                downloadId: downloadId.to_string(),
                fileId: file.fileId.clone(),
                state: HttpDownloadProgressState::Downloading,
                fileDownloadedBytes,
                fileTotalBytes: file.expectedBytes,
                downloadedBytes: 0,
                totalBytes,
                completedFiles: 0,
                totalFiles,
            },
        )?;
    }
    output
        .flush()
        .map_err(|error| HostError::new(error.to_string()))?;
    if fileDownloadedBytes != file.expectedBytes {
        return Err(HostError::new(format!(
            "HTTP download size mismatch: {} expected={} actual={}",
            file.fileId, file.expectedBytes, fileDownloadedBytes
        )));
    }
    completedFiles.fetch_add(1, Ordering::SeqCst);
    fs::rename(partialTarget, target).map_err(|error| HostError::new(error.to_string()))?;
    publishDownloadProgress(
        progressGate,
        downloadedBytes,
        completedFiles,
        onProgress,
        HttpDownloadProgress {
            downloadId: downloadId.to_string(),
            fileId: file.fileId.clone(),
            state: HttpDownloadProgressState::Completed,
            fileDownloadedBytes,
            fileTotalBytes: file.expectedBytes,
            downloadedBytes: 0,
            totalBytes,
            completedFiles: 0,
            totalFiles,
        },
    )?;
    Ok(HttpDownloadFileResult {
        fileId: file.fileId.clone(),
        finalUrl,
        targetPath: file.targetPath.clone(),
        downloadedBytes: fileDownloadedBytes,
    })
}

/// Resolves after a control token requests cancellation without blocking a network task.
async fn waitForDownloadCancellation(control: &HttpDownloadControl) {
    while !control.isCancelled() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Reconciles final and partial files before one resumable HTTP request starts.
fn prepareResumableDownload(
    target: &Path,
    partialTarget: &Path,
    expectedBytes: u64,
) -> HostResult<u64> {
    if target.exists() {
        let targetBytes = fs::metadata(target)
            .map_err(|error| HostError::new(error.to_string()))?
            .len();
        if targetBytes == expectedBytes {
            if partialTarget.exists() {
                fs::remove_file(partialTarget)
                    .map_err(|error| HostError::new(error.to_string()))?;
            }
            return Ok(targetBytes);
        }
        fs::remove_file(target).map_err(|error| HostError::new(error.to_string()))?;
    }
    if !partialTarget.exists() {
        return Ok(0);
    }
    let partialBytes = fs::metadata(partialTarget)
        .map_err(|error| HostError::new(error.to_string()))?
        .len();
    if partialBytes > expectedBytes {
        fs::remove_file(partialTarget).map_err(|error| HostError::new(error.to_string()))?;
        return Err(HostError::new(format!(
            "HTTP partial download exceeds declared byte count: expected={expectedBytes} actual={partialBytes}"
        )));
    }
    Ok(partialBytes)
}

/// Validates the exact byte interval returned for a resumed HTTP download.
fn validateContentRange(
    headers: &HeaderMap,
    expectedStart: u64,
    expectedTotal: u64,
) -> HostResult<()> {
    let value = headers
        .get(CONTENT_RANGE)
        .ok_or_else(|| HostError::new("HTTP partial download response has no Content-Range"))?
        .to_str()
        .map_err(|error| HostError::new(error.to_string()))?;
    let (unit, range) = value
        .split_once(' ')
        .ok_or_else(|| HostError::new("HTTP Content-Range format is invalid"))?;
    if unit != "bytes" {
        return Err(HostError::new("HTTP Content-Range unit is not bytes"));
    }
    let (interval, total) = range
        .split_once('/')
        .ok_or_else(|| HostError::new("HTTP Content-Range total is missing"))?;
    let (start, end) = interval
        .split_once('-')
        .ok_or_else(|| HostError::new("HTTP Content-Range interval is invalid"))?;
    let start = start
        .parse::<u64>()
        .map_err(|error| HostError::new(error.to_string()))?;
    let end = end
        .parse::<u64>()
        .map_err(|error| HostError::new(error.to_string()))?;
    let total = total
        .parse::<u64>()
        .map_err(|error| HostError::new(error.to_string()))?;
    if start != expectedStart || total != expectedTotal || end != expectedTotal - 1 {
        return Err(HostError::new(format!(
            "HTTP Content-Range does not match the requested resume: {value}"
        )));
    }
    Ok(())
}

/// Serializes callbacks and refreshes aggregate counters at publication time.
fn publishDownloadProgress(
    progressGate: &Mutex<()>,
    downloadedBytes: &AtomicU64,
    completedFiles: &AtomicUsize,
    onProgress: &HttpDownloadProgressCallback,
    mut progress: HttpDownloadProgress,
) -> HostResult<()> {
    let _guard = progressGate.lock().map_err(|error| {
        HostError::new(format!("HTTP download progress lock poisoned: {error}"))
    })?;
    progress.downloadedBytes = downloadedBytes.load(Ordering::SeqCst);
    progress.completedFiles = completedFiles.load(Ordering::SeqCst);
    onProgress(progress);
    Ok(())
}

/// Returns whether another worker has recorded a terminal error.
fn downloadHasFailed(failure: &Mutex<Option<HostError>>) -> bool {
    match failure.lock() {
        Ok(error) => error.is_some(),
        Err(_) => true,
    }
}

/// Records the first terminal worker error.
fn recordDownloadFailure(failure: &Mutex<Option<HostError>>, error: HostError) {
    if let Ok(mut current) = failure.lock() {
        if current.is_none() {
            *current = Some(error);
        }
    }
}

/// Removes and returns the terminal worker error.
fn takeDownloadFailure(failure: &Mutex<Option<HostError>>) -> HostResult<Option<HostError>> {
    failure
        .lock()
        .map(|mut error| error.take())
        .map_err(|error| HostError::new(format!("HTTP download failure lock poisoned: {error}")))
}

/// Removes one incomplete target file after a worker error.
fn removeIncompleteDownload(targetPath: &str) -> HostResult<()> {
    let target = Path::new(targetPath);
    if target.is_file() {
        fs::remove_file(target).map_err(|error| HostError::new(error.to_string()))?;
    }
    Ok(())
}

/// Removes every target that does not have a completed result.
fn removeQueuedAndIncompleteTargets(
    files: &[HttpDownloadFileRequest],
    results: &Mutex<BTreeMap<String, HttpDownloadFileResult>>,
) -> HostResult<()> {
    let completed = results
        .lock()
        .map_err(|error| HostError::new(format!("HTTP download result lock poisoned: {error}")))?;
    for file in files {
        if !completed.contains_key(&file.fileId) {
            removeIncompleteDownload(&file.targetPath)?;
        }
    }
    Ok(())
}

/// Converts request header pairs into reqwest headers.
fn headersToReqwest(headers: &[(String, String)]) -> HostResult<HeaderMap> {
    let mut result = HeaderMap::new();
    for (name, value) in headers {
        let headerName = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| HostError::new(error.to_string()))?;
        let headerValue =
            HeaderValue::from_str(value).map_err(|error| HostError::new(error.to_string()))?;
        result.insert(headerName, headerValue);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::sync::Barrier;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// Verifies successive buffered requests share the same live HTTP connection.
    #[tokio::test(flavor = "current_thread")]
    async fn bufferedRequestsReuseConnection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            for _ in 0..2 {
                let request = readHttpRequest(&mut stream);
                assert!(request.starts_with("GET /reuse "));
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                    .unwrap();
            }
        });
        let host = NativeHttpHost::new();
        for _ in 0..2 {
            let response = host
                .executeHttpRequest(HttpRequestData {
                    url: format!("http://{address}/reuse"),
                    method: "GET".to_string(),
                    headers: Vec::new(),
                    body: Vec::new(),
                    formFields: Vec::new(),
                    fileParts: Vec::new(),
                    connectTimeoutSeconds: 2,
                    readTimeoutSeconds: 2,
                    followRedirects: false,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                })
                .unwrap();
            assert_eq!(response.body, b"ok");
        }
        server.join().unwrap();
    }

    /// Verifies a direct file download publishes only the completed destination.
    #[test]
    fn downloadsResponseDirectlyToFile() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = readHttpRequest(&mut stream);
            assert!(request.starts_with("GET /file "));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello world")
                .unwrap();
        });
        let directory = uniqueTempDir("direct-file");
        let target = directory.join("artifact.bin");
        let response = NativeHttpHost::new()
            .downloadToFile(
                HttpRequestData {
                    url: format!("http://{address}/file"),
                    method: "GET".to_string(),
                    headers: Vec::new(),
                    body: Vec::new(),
                    formFields: Vec::new(),
                    fileParts: Vec::new(),
                    connectTimeoutSeconds: 2,
                    readTimeoutSeconds: 2,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                target.to_string_lossy().to_string(),
            )
            .unwrap();
        assert_eq!(response.downloadedBytes, 11);
        assert_eq!(fs::read(&target).unwrap(), b"hello world");
        assert!(!target.with_extension("bin.partial").exists());
        server.join().unwrap();
        let _ = fs::remove_dir_all(directory);
    }

    /// Verifies the native WebSocket carrier keeps polling until a delayed server message arrives.
    #[test]
    fn websocketReceivesMessageAfterReadTimeoutTick() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            std::thread::sleep(Duration::from_millis(1300));
            socket
                .send(tungstenite::Message::Binary(b"late-message".to_vec()))
                .unwrap();
            let _ = socket.close(None);
        });

        let host = NativeHttpHost::new();
        let (openedSender, openedReceiver) = mpsc::channel();
        let (messageSender, messageReceiver) = mpsc::channel();
        let (closedSender, closedReceiver) = mpsc::channel();
        host.openWebSocket(
            "websocket-timeout-test".to_string(),
            WebSocketRequestData {
                url: format!("ws://{address}"),
                headers: Vec::new(),
                connectTimeoutSeconds: 1,
                ignoreSsl: false,
            },
            Arc::new(move || {
                openedSender.send(()).unwrap();
            }),
            Arc::new(move |message| {
                messageSender.send(message).unwrap();
            }),
            Arc::new(move |result| {
                closedSender.send(result).unwrap();
            }),
        )
        .unwrap();

        openedReceiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(
            messageReceiver
                .recv_timeout(Duration::from_secs(3))
                .unwrap(),
            b"late-message".to_vec()
        );
        assert!(closedReceiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
            .is_ok());
        server.join().unwrap();
    }

    /// An idle inbound socket must not delay a newly queued outbound frame by
    /// the connection timeout (the Link handshake waits for this very frame).
    #[test]
    fn websocketSendsPromptlyWhileWaitingForAnInboundMessage() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            assert_eq!(
                socket.read().unwrap(),
                tungstenite::Message::Binary(b"queued-message".to_vec())
            );
            socket
                .send(tungstenite::Message::Binary(b"received".to_vec()))
                .unwrap();
            let _ = socket.close(None);
        });
        let host = NativeHttpHost::new();
        let (openedSender, openedReceiver) = mpsc::channel();
        let (messageSender, messageReceiver) = mpsc::channel();
        let (closedSender, closedReceiver) = mpsc::channel();
        host.openWebSocket(
            "websocket-send-latency-test".into(),
            WebSocketRequestData {
                url: format!("ws://{address}"),
                headers: Vec::new(),
                connectTimeoutSeconds: 15,
                ignoreSsl: false,
            },
            Arc::new(move || {
                openedSender.send(()).unwrap();
            }),
            Arc::new(move |message| {
                let _ = messageSender.send(message);
            }),
            Arc::new(move |result| {
                let _ = closedSender.send(result);
            }),
        )
        .unwrap();
        openedReceiver.recv_timeout(Duration::from_secs(3)).unwrap();
        // Let the network thread enter its read before sending a command.
        std::thread::sleep(Duration::from_millis(100));
        host.sendWebSocketMessage("websocket-send-latency-test", b"queued-message".to_vec())
            .unwrap();
        let message = messageReceiver.recv_timeout(Duration::from_millis(750));
        // Close even on a timing assertion failure, so no host thread leaks.
        let _ = host.closeWebSocket("websocket-send-latency-test");
        server.join().unwrap();
        assert_eq!(message.unwrap(), b"received");
        assert!(closedReceiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
            .is_ok());
    }

    /// Verifies two files enter the server concurrently and publish aggregate progress.
    #[test]
    fn downloadsFilesWithBoundedParallelWorkers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let serverBarrier = barrier.clone();
        let server = std::thread::spawn(move || {
            let mut handlers = Vec::new();
            for _ in 0..2 {
                let (stream, _) = listener.accept().unwrap();
                let connectionBarrier = serverBarrier.clone();
                handlers.push(std::thread::spawn(move || {
                    serveDownloadConnection(stream, connectionBarrier)
                }));
            }
            for handler in handlers {
                handler.join().unwrap();
            }
        });
        let root = uniqueTempDir("parallel");
        let progress = Arc::new(Mutex::new(Vec::<HttpDownloadProgress>::new()));
        let progressEvents = progress.clone();
        let result = NativeHttpHost::new()
            .downloadFiles(
                HttpDownloadRequest {
                    downloadId: "parallel-test".to_string(),
                    files: vec![
                        HttpDownloadFileRequest {
                            fileId: "a".to_string(),
                            url: format!("http://{address}/a"),
                            targetPath: root.join("a.bin").to_string_lossy().to_string(),
                            headers: Vec::new(),
                            expectedBytes: 4,
                        },
                        HttpDownloadFileRequest {
                            fileId: "b".to_string(),
                            url: format!("http://{address}/b"),
                            targetPath: root.join("b.bin").to_string_lossy().to_string(),
                            headers: Vec::new(),
                            expectedBytes: 4,
                        },
                    ],
                    maxConcurrency: 2,
                    connectTimeoutSeconds: 5,
                    readTimeoutSeconds: 5,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                HttpDownloadControl::new(),
                Arc::new(move |event| {
                    progressEvents.lock().unwrap().push(event);
                }),
            )
            .unwrap();
        server.join().unwrap();

        assert_eq!(result.files.len(), 2);
        assert_eq!(result.downloadedBytes, 8);
        assert_eq!(fs::read(root.join("a.bin")).unwrap(), b"aaaa");
        assert_eq!(fs::read(root.join("b.bin")).unwrap(), b"bbbb");
        assert_eq!(
            progress
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.state == HttpDownloadProgressState::Completed)
                .count(),
            2
        );
        assert!(progress
            .lock()
            .unwrap()
            .windows(2)
            .all(|events| events[0].downloadedBytes <= events[1].downloadedBytes));
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies a cancelled operation exits before opening a network request or target file.
    #[test]
    fn cancellationStopsDownloadBeforeFileCreation() {
        let root = uniqueTempDir("cancelled");
        let target = root.join("cancelled.bin");
        let control = HttpDownloadControl::new();
        control.cancel();
        let error = NativeHttpHost::new()
            .downloadFiles(
                HttpDownloadRequest {
                    downloadId: "cancelled-test".to_string(),
                    files: vec![HttpDownloadFileRequest {
                        fileId: "cancelled".to_string(),
                        url: "http://127.0.0.1:9/cancelled".to_string(),
                        targetPath: target.to_string_lossy().to_string(),
                        headers: Vec::new(),
                        expectedBytes: 4,
                    }],
                    maxConcurrency: 1,
                    connectTimeoutSeconds: 1,
                    readTimeoutSeconds: 1,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                control,
                Arc::new(|_| {}),
            )
            .expect_err("cancelled download must fail");

        assert_eq!(error.message, "HTTP download cancelled: cancelled-test");
        assert!(!target.exists());
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies a retained partial file resumes through an exact HTTP byte range.
    #[test]
    fn resumesRetainedPartialDownload() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let requestText = readHttpRequest(&mut stream);
            let range = requestText
                .lines()
                .find_map(|line| line.split_once(':'))
                .filter(|(name, _)| name.eq_ignore_ascii_case("range"))
                .map(|(_, value)| value.trim());
            assert_eq!(range, Some("bytes=2-"));
            stream.write_all(
                b"HTTP/1.1 206 Partial Content\r\nContent-Length: 2\r\nContent-Range: bytes 2-3/4\r\nConnection: close\r\n\r\ncd",
            ).unwrap();
            stream.flush().unwrap();
        });
        let root = uniqueTempDir("resume");
        let target = root.join("resume.bin");
        let partial = httpDownloadPartialTargetPath(&target.to_string_lossy());
        fs::write(&partial, b"ab").unwrap();
        let progress = Arc::new(Mutex::new(Vec::<HttpDownloadProgress>::new()));
        let progressEvents = progress.clone();
        let result = NativeHttpHost::new()
            .downloadFiles(
                HttpDownloadRequest {
                    downloadId: "resume-test".to_string(),
                    files: vec![HttpDownloadFileRequest {
                        fileId: "resume".to_string(),
                        url: format!("http://{address}/resume"),
                        targetPath: target.to_string_lossy().to_string(),
                        headers: Vec::new(),
                        expectedBytes: 4,
                    }],
                    maxConcurrency: 1,
                    connectTimeoutSeconds: 5,
                    readTimeoutSeconds: 5,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                HttpDownloadControl::new(),
                Arc::new(move |event| {
                    progressEvents.lock().unwrap().push(event);
                }),
            )
            .unwrap();
        server.join().unwrap();

        assert_eq!(result.downloadedBytes, 4);
        assert_eq!(fs::read(&target).unwrap(), b"abcd");
        assert!(!Path::new(&partial).exists());
        assert!(progress.lock().unwrap().iter().any(|event| {
            event.state == HttpDownloadProgressState::Started && event.fileDownloadedBytes == 2
        }));
        fs::remove_dir_all(root).unwrap();
    }

    /// Verifies cancellation interrupts a response whose next body chunk is stalled.
    #[test]
    fn cancellationInterruptsStalledResponseRead() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (headersSent, headersReceived) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            readHttpRequest(&mut stream);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n")
                .unwrap();
            stream.flush().unwrap();
            headersSent.send(()).unwrap();
            std::thread::sleep(Duration::from_secs(1));
        });
        let root = uniqueTempDir("cancel-stalled-read");
        let target = root.join("stalled.bin");
        let control = HttpDownloadControl::new();
        let taskControl = control.clone();
        let task = std::thread::spawn(move || {
            NativeHttpHost::new().downloadFiles(
                HttpDownloadRequest {
                    downloadId: "stalled-read-test".to_string(),
                    files: vec![HttpDownloadFileRequest {
                        fileId: "stalled".to_string(),
                        url: format!("http://{address}/stalled"),
                        targetPath: target.to_string_lossy().to_string(),
                        headers: Vec::new(),
                        expectedBytes: 4,
                    }],
                    maxConcurrency: 1,
                    connectTimeoutSeconds: 5,
                    readTimeoutSeconds: 5,
                    followRedirects: true,
                    ignoreSsl: false,
                    proxyHost: String::new(),
                    proxyPort: 0,
                },
                taskControl,
                Arc::new(|_| {}),
            )
        });
        headersReceived.recv().unwrap();
        let started = Instant::now();
        control.cancel();
        let error = task
            .join()
            .unwrap()
            .expect_err("cancelled download must fail");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(error.message, "HTTP download cancelled: stalled-read-test");
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// Serves one four-byte test file after both worker requests have arrived.
    fn serveDownloadConnection(mut stream: TcpStream, barrier: Arc<Barrier>) {
        let requestText = readHttpRequest(&mut stream);
        let body = if requestText.starts_with("GET /a ") {
            b"aaaa".as_slice()
        } else if requestText.starts_with("GET /b ") {
            b"bbbb".as_slice()
        } else {
            panic!("unexpected HTTP request: {requestText}");
        };
        barrier.wait();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n")
            .unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
    }

    /// Reads one complete HTTP request header block from a test connection.
    fn readHttpRequest(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            request.extend_from_slice(&buffer[..read]);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(request).unwrap()
    }

    /// Creates a unique native directory for one download test.
    fn uniqueTempDir(label: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "operit-http-download-{label}-{}-{now}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}

#[cfg(test)]
mod response_stream_tests;
