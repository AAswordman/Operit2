use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use operit_host_api::{
    httpDownloadPartialTargetPath, HostError, HostResult, HttpDownloadControl,
    HttpDownloadFileRequest, HttpDownloadFileResult, HttpDownloadProgress,
    HttpDownloadProgressCallback, HttpDownloadProgressState, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpRequestData, HttpResponseData,
};
use reqwest::blocking::{multipart, Client as BlockingClient};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_RANGE, RANGE};
use reqwest::{Client as AsyncClient, Method, Proxy, StatusCode};

#[derive(Clone, Debug, Default)]
pub struct NativeHttpHost;

impl NativeHttpHost {
    /// Creates a native HTTP host.
    pub fn new() -> Self {
        Self
    }
}

impl HttpHost for NativeHttpHost {
    /// Executes a buffered request on a dedicated native HTTP thread.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        std::thread::spawn(move || executeHttpRequestOnBlockingThread(request))
            .join()
            .map_err(|_| HostError::new("native HTTP request thread panicked"))?
    }

    /// Downloads files on a dedicated manager thread with bounded worker concurrency.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        std::thread::spawn(move || executeDownloadBatch(request, control, onProgress))
            .join()
            .map_err(|_| HostError::new("native HTTP download manager thread panicked"))?
    }
}

/// Executes one buffered request outside every caller-owned async runtime context.
fn executeHttpRequestOnBlockingThread(request: HttpRequestData) -> HostResult<HttpResponseData> {
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|error| HostError::new(error.to_string()))?;
    let client = buildHttpClient(
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
        let mut form = multipart::Form::new();
        for (name, value) in request.formFields {
            form = form.text(name, value);
        }
        for file in request.fileParts {
            let part = multipart::Part::bytes(file.content)
                .file_name(file.fileName)
                .mime_str(&file.contentType)
                .map_err(|error| HostError::new(error.to_string()))?;
            form = form.part(file.fieldName, part);
        }
        httpRequest = httpRequest.multipart(form);
    } else if !request.body.is_empty() {
        httpRequest = httpRequest.body(request.body);
    }
    let response = httpRequest
        .send()
        .map_err(|error| HostError::new(error.to_string()))?;
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
    let body = response
        .bytes()
        .map_err(|error| HostError::new(error.to_string()))?
        .to_vec();
    Ok(HttpResponseData {
        finalUrl,
        statusCode,
        statusMessage,
        headers,
        body,
    })
}

/// Executes one validated batch through a bounded native worker pool.
fn executeDownloadBatch(
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
    let queue = Arc::new(Mutex::new(VecDeque::from(request.files.clone())));
    let results = Arc::new(Mutex::new(BTreeMap::<String, HttpDownloadFileResult>::new()));
    let failure = Arc::new(Mutex::new(None::<HostError>));
    let downloadedBytes = Arc::new(AtomicU64::new(0));
    let completedFiles = Arc::new(AtomicUsize::new(0));
    let progressGate = Arc::new(Mutex::new(()));

    std::thread::scope(|scope| {
        for _ in 0..workerCount {
            let client = client.clone();
            let queue = queue.clone();
            let results = results.clone();
            let failure = failure.clone();
            let downloadedBytes = downloadedBytes.clone();
            let completedFiles = completedFiles.clone();
            let progressGate = progressGate.clone();
            let control = control.clone();
            let onProgress = onProgress.clone();
            let downloadId = request.downloadId.clone();
            scope.spawn(move || loop {
                if control.isCancelled() || downloadHasFailed(&failure) {
                    break;
                }
                let file = match nextDownloadFile(&queue) {
                    Ok(file) => file,
                    Err(error) => {
                        recordDownloadFailure(&failure, error);
                        break;
                    }
                };
                let Some(file) = file else {
                    break;
                };
                let result = downloadOneFile(
                    &client,
                    &downloadId,
                    &file,
                    totalBytes,
                    totalFiles,
                    &downloadedBytes,
                    &completedFiles,
                    &progressGate,
                    &control,
                    &failure,
                    &onProgress,
                );
                match result {
                    Ok(result) => match results.lock() {
                        Ok(mut entries) => {
                            entries.insert(result.fileId.clone(), result);
                        }
                        Err(error) => {
                            recordDownloadFailure(
                                &failure,
                                HostError::new(format!(
                                    "HTTP download result lock poisoned: {error}"
                                )),
                            );
                            break;
                        }
                    },
                    Err(error) => {
                        recordDownloadFailure(&failure, error);
                        break;
                    }
                }
            });
        }
    });

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

/// Builds one reqwest client from an explicit Host request policy.
fn buildHttpClient(
    connectTimeoutSeconds: u64,
    readTimeoutSeconds: u64,
    followRedirects: bool,
    ignoreSsl: bool,
    proxyHost: &str,
    proxyPort: u16,
) -> HostResult<BlockingClient> {
    let mut builder = BlockingClient::builder()
        .connect_timeout(Duration::from_secs(connectTimeoutSeconds))
        .timeout(Duration::from_secs(readTimeoutSeconds))
        .danger_accept_invalid_certs(ignoreSsl);
    if !followRedirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    if !proxyHost.trim().is_empty() && proxyPort > 0 {
        let proxyUrl = format!("http://{}:{}", proxyHost.trim(), proxyPort);
        builder = builder
            .proxy(Proxy::http(&proxyUrl).map_err(|error| HostError::new(error.to_string()))?);
    }
    builder
        .build()
        .map_err(|error| HostError::new(error.to_string()))
}

/// Builds one asynchronous client so an active request can observe cancellation immediately.
fn buildDownloadHttpClient(
    connectTimeoutSeconds: u64,
    readTimeoutSeconds: u64,
    followRedirects: bool,
    ignoreSsl: bool,
    proxyHost: &str,
    proxyPort: u16,
) -> HostResult<AsyncClient> {
    let mut builder = AsyncClient::builder()
        .connect_timeout(Duration::from_secs(connectTimeoutSeconds))
        .timeout(Duration::from_secs(readTimeoutSeconds))
        .danger_accept_invalid_certs(ignoreSsl);
    if !followRedirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    if !proxyHost.trim().is_empty() && proxyPort > 0 {
        let proxyUrl = format!("http://{}:{}", proxyHost.trim(), proxyPort);
        builder = builder
            .proxy(Proxy::http(&proxyUrl).map_err(|error| HostError::new(error.to_string()))?);
    }
    builder
        .build()
        .map_err(|error| HostError::new(error.to_string()))
}

/// Removes and returns the next queued file.
fn nextDownloadFile(
    queue: &Mutex<VecDeque<HttpDownloadFileRequest>>,
) -> HostResult<Option<HttpDownloadFileRequest>> {
    queue
        .lock()
        .map(|mut files| files.pop_front())
        .map_err(|error| HostError::new(format!("HTTP download queue lock poisoned: {error}")))
}

/// Downloads one file into its durable partial target and publishes aggregate progress.
fn downloadOneFile(
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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| HostError::new(error.to_string()))?;
    runtime.block_on(downloadOneFileAsync(
        client,
        downloadId,
        file,
        totalBytes,
        totalFiles,
        downloadedBytes,
        completedFiles,
        progressGate,
        control,
        failure,
        onProgress,
    ))
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
        response = request.send() => response.map_err(|error| HostError::new(error.to_string()))?,
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
