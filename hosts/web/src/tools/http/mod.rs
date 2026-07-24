use std::cell::RefCell;
use std::collections::HashMap;

use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadFileRequest, HttpDownloadFileResult,
    HttpDownloadProgress, HttpDownloadProgressCallback, HttpDownloadProgressState,
    HttpDownloadRequest, HttpDownloadResult, HttpHost, HttpRequestData, HttpResponseData,
    HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost, HttpStreamOpenedCallback,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use crate::common::{
    call_http, http_request_to_js, js_http_response, read_i64_property, read_string_property,
    set_property, string_pairs_to_js,
};

/// Retains browser callback closures until the associated HTTP byte stream closes.
struct WebHttpByteStreamCallbacks {
    _opened: Closure<dyn FnMut()>,
    _chunk: Closure<dyn FnMut(JsValue)>,
    _closed: Closure<dyn FnMut(JsValue)>,
}

thread_local! {
    static HTTP_BYTE_STREAM_CALLBACKS: RefCell<HashMap<String, WebHttpByteStreamCallbacks>> =
        RefCell::new(HashMap::new());
}

#[derive(Clone, Debug, Default)]
pub struct WebHttpHost;

impl WebHttpHost {
    /// Creates the web HTTP host.
    pub fn new() -> Self {
        Self
    }
}

impl HttpHost for WebHttpHost {
    /// Executes one buffered HTTP request through the installed JavaScript host bridge.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        js_http_response(call_http(
            "executeHttpRequest",
            &[http_request_to_js(request)],
        )?)
    }

    /// Downloads files through the browser host and writes them into runtime storage.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        let totalBytes = request
            .files
            .iter()
            .map(|file| file.expectedBytes)
            .sum::<u64>();
        let totalFiles = request.files.len();
        let mut downloadedBytes = 0u64;
        let mut completedFiles = 0usize;
        let mut results = Vec::new();
        for file in &request.files {
            control.checkpoint(&request.downloadId, &file.fileId)?;
            onProgress(HttpDownloadProgress {
                downloadId: request.downloadId.clone(),
                fileId: file.fileId.clone(),
                state: HttpDownloadProgressState::Started,
                fileDownloadedBytes: 0,
                fileTotalBytes: file.expectedBytes,
                downloadedBytes,
                totalBytes,
                completedFiles,
                totalFiles,
            });
            let result = js_download_file(&request, file)?;
            downloadedBytes += result.downloadedBytes;
            completedFiles += 1;
            onProgress(HttpDownloadProgress {
                downloadId: request.downloadId.clone(),
                fileId: file.fileId.clone(),
                state: HttpDownloadProgressState::Completed,
                fileDownloadedBytes: result.downloadedBytes,
                fileTotalBytes: file.expectedBytes,
                downloadedBytes,
                totalBytes,
                completedFiles,
                totalFiles,
            });
            results.push(result);
        }
        Ok(HttpDownloadResult {
            downloadId: request.downloadId,
            files: results,
            downloadedBytes,
        })
    }
}

impl HttpStreamHost for WebHttpHost {
    /// Opens one browser Fetch response body and forwards its ordered byte chunks to Rust.
    #[allow(non_snake_case)]
    fn openHttpByteStream(
        &self,
        streamId: String,
        request: HttpRequestData,
        onOpened: HttpStreamOpenedCallback,
        onChunk: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let opened = Closure::wrap(Box::new(move || onOpened()) as Box<dyn FnMut()>);
        let chunk = Closure::wrap(Box::new(move |value: JsValue| {
            onChunk(js_sys::Uint8Array::new(&value).to_vec());
        }) as Box<dyn FnMut(JsValue)>);
        let closedStreamId = streamId.clone();
        let closed = Closure::wrap(Box::new(move |value: JsValue| {
            let result = if value.is_null() || value.is_undefined() {
                Ok(())
            } else {
                Err(value.as_string().unwrap_or_else(|| format!("{value:?}")))
            };
            onClosed(result);
            HTTP_BYTE_STREAM_CALLBACKS.with(|streams| {
                streams.borrow_mut().remove(&closedStreamId);
            });
        }) as Box<dyn FnMut(JsValue)>);
        let args = [
            JsValue::from_str(&streamId),
            http_request_to_js(request),
            opened.as_ref().clone(),
            chunk.as_ref().clone(),
            closed.as_ref().clone(),
        ];
        HTTP_BYTE_STREAM_CALLBACKS.with(|streams| {
            let mut streams = streams.borrow_mut();
            if streams.contains_key(&streamId) {
                return Err(HostError::new(format!(
                    "HTTP byte stream is already open: {streamId}"
                )));
            }
            streams.insert(
                streamId.clone(),
                WebHttpByteStreamCallbacks {
                    _opened: opened,
                    _chunk: chunk,
                    _closed: closed,
                },
            );
            if let Err(error) = call_http("openHttpByteStream", &args) {
                streams.remove(&streamId);
                return Err(error);
            }
            Ok(())
        })
    }

    /// Requests cancellation for one browser-owned HTTP byte stream.
    #[allow(non_snake_case)]
    fn closeHttpByteStream(&self, streamId: &str) -> HostResult<()> {
        call_http("closeHttpByteStream", &[JsValue::from_str(streamId)])?;
        Ok(())
    }
}

/// Converts cancellation into the shared host error shape.
trait WebDownloadControlExt {
    /// Returns an error when the caller cancelled the current download.
    fn checkpoint(&self, downloadId: &str, fileId: &str) -> HostResult<()>;
}

impl WebDownloadControlExt for HttpDownloadControl {
    fn checkpoint(&self, downloadId: &str, fileId: &str) -> HostResult<()> {
        if self.isCancelled() {
            return Err(operit_host_api::HostError::new(format!(
                "HTTP download cancelled: {downloadId}/{fileId}"
            )));
        }
        Ok(())
    }
}

/// Downloads one file through the installed JavaScript browser bridge.
fn js_download_file(
    request: &HttpDownloadRequest,
    file: &HttpDownloadFileRequest,
) -> HostResult<HttpDownloadFileResult> {
    let value = call_http(
        "downloadFile",
        &[http_download_request_to_js(request, file)],
    )?;
    Ok(HttpDownloadFileResult {
        fileId: read_string_property(&value, "fileId")?,
        finalUrl: read_string_property(&value, "finalUrl")?,
        targetPath: read_string_property(&value, "targetPath")?,
        downloadedBytes: read_i64_property(&value, "downloadedBytes")? as u64,
    })
}

/// Builds the browser-side download request object for one target file.
fn http_download_request_to_js(
    request: &HttpDownloadRequest,
    file: &HttpDownloadFileRequest,
) -> JsValue {
    let object = js_sys::Object::new();
    set_property(
        &object,
        "downloadId",
        JsValue::from_str(&request.downloadId),
    );
    set_property(&object, "fileId", JsValue::from_str(&file.fileId));
    set_property(&object, "url", JsValue::from_str(&file.url));
    set_property(&object, "targetPath", JsValue::from_str(&file.targetPath));
    set_property(&object, "headers", string_pairs_to_js(&file.headers));
    set_property(
        &object,
        "expectedBytes",
        JsValue::from_f64(file.expectedBytes as f64),
    );
    set_property(
        &object,
        "connectTimeoutSeconds",
        JsValue::from_f64(request.connectTimeoutSeconds as f64),
    );
    set_property(
        &object,
        "readTimeoutSeconds",
        JsValue::from_f64(request.readTimeoutSeconds as f64),
    );
    set_property(
        &object,
        "followRedirects",
        JsValue::from_bool(request.followRedirects),
    );
    object.into()
}
