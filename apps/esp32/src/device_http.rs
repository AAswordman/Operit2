//! HTTP transport for native device applications. Streaming stays off the UI task.
#![allow(non_snake_case)]
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection, FollowRedirectsPolicy, Method};
use operit_host_api::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct StreamState {
    cancelled: AtomicBool,
    handle: Mutex<usize>,
}
// Clear the handle before EspHttpConnection is dropped. Cancellation holds the
// same mutex while interrupting I/O, so it cannot race connection destruction.
struct RegisteredHandle<'a>(&'a StreamState);
impl Drop for RegisteredHandle<'_> {
    fn drop(&mut self) {
        *self.0.handle.lock().unwrap() = 0;
    }
}
#[derive(Default)]
pub struct DeviceHttpHost {
    streams: Arc<Mutex<BTreeMap<String, Arc<StreamState>>>>,
}
fn run(
    request: HttpRequestData,
    state: &StreamState,
    head: HttpStreamResponseCallback,
    chunk: HttpStreamChunkCallback,
) -> HostResult<()> {
    if request.ignoreSsl
        || !request.proxyHost.is_empty()
        || !request.formFields.is_empty()
        || !request.fileParts.is_empty()
    {
        return Err(HostError::new("Unsupported device HTTP options"));
    }
    let method = match request.method.as_str() {
        "GET" => Method::Get,
        "POST" => Method::Post,
        _ => return Err(HostError::new("Unsupported device HTTP method")),
    };
    let mut connection = EspHttpConnection::new(&Configuration {
        buffer_size: Some(1024),
        buffer_size_tx: Some(1024),
        timeout: Some(std::time::Duration::from_secs(
            request.readTimeoutSeconds.clamp(1, 45),
        )),
        follow_redirects_policy: FollowRedirectsPolicy::FollowNone,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })
    .map_err(|e| HostError::new(e.to_string()))?;
    *state.handle.lock().unwrap() = connection.handle() as usize;
    let _registered = RegisteredHandle(state);
    let mut headers = request.headers;
    headers.push(("Content-Length".into(), request.body.len().to_string()));
    let borrowed: Vec<_> = headers
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    if state.cancelled.load(Ordering::Acquire) {
        return Err(HostError::new("HTTP request cancelled"));
    }
    connection
        .initiate_request(method, &request.url, &borrowed)
        .map_err(|e| HostError::new(e.to_string()))?;
    connection
        .write_all(&request.body)
        .map_err(|e| HostError::new(e.to_string()))?;
    connection
        .initiate_response()
        .map_err(|e| HostError::new(e.to_string()))?;
    head(HttpResponseHead {
        finalUrl: request.url,
        statusCode: connection.status() as i32,
        statusMessage: connection.status_message().unwrap_or("").into(),
        headers: vec![],
    });
    let mut buffer = [0u8; 1024];
    loop {
        if state.cancelled.load(Ordering::Acquire) {
            return Err(HostError::new("HTTP request cancelled"));
        }
        let read = connection
            .read(&mut buffer)
            .map_err(|e| HostError::new(e.to_string()))?;
        if read == 0 {
            break;
        }
        chunk(buffer[..read].to_vec());
    }
    Ok(())
}
impl HttpHost for DeviceHttpHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new(
            "File downloads are not provided by this device host",
        ))
    }
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        Err(HostError::new(
            "Device applications must use bounded HTTP streaming",
        ))
    }
}
impl HttpStreamHost for DeviceHttpHost {
    fn openHttpByteStream(
        &self,
        id: String,
        request: HttpRequestData,
        opened: HttpStreamOpenedCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let success = Arc::new(AtomicBool::new(false));
        let headSuccess = success.clone();
        let chunkSuccess = success.clone();
        self.openHttpResponseStream(
            id,
            request,
            Arc::new(move |head| {
                if (200..300).contains(&head.statusCode) {
                    headSuccess.store(true, Ordering::Release);
                    opened();
                }
            }),
            Arc::new(move |bytes| {
                if chunkSuccess.load(Ordering::Acquire) {
                    chunk(bytes);
                }
            }),
            Arc::new(move |result| {
                closed(result.and_then(|_| {
                    if success.load(Ordering::Acquire) {
                        Ok(())
                    } else {
                        Err("HTTP request failed".into())
                    }
                }));
            }),
        )
    }
    fn openHttpResponseStream(
        &self,
        id: String,
        request: HttpRequestData,
        head: HttpStreamResponseCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        let state = Arc::new(StreamState::default());
        {
            let mut streams = self.streams.lock().unwrap();
            if !streams.is_empty() {
                return Err(HostError::new("Device HTTP is busy"));
            }
            streams.insert(id.clone(), state.clone());
        }
        let streams = self.streams.clone();
        let workerId = id.clone();
        let started = std::thread::Builder::new()
            .name("device-http".into())
            .stack_size(16 * 1024)
            .spawn(move || {
                let result = run(request, &state, head, chunk).map_err(|e| e.to_string());
                streams.lock().unwrap().remove(&workerId);
                closed(result);
            });
        if let Err(error) = started {
            self.streams.lock().unwrap().remove(&id);
            return Err(HostError::new(error.to_string()));
        }
        Ok(())
    }
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        let state = self.streams.lock().unwrap().get(id).cloned();
        if let Some(state) = state {
            state.cancelled.store(true, Ordering::Release);
            let handle = state.handle.lock().unwrap();
            if *handle != 0 {
                // ESP-IDF exposes this specifically to interrupt ongoing HTTP I/O.
                unsafe {
                    esp_idf_svc::sys::esp_http_client_cancel_request(*handle as _);
                }
            }
        }
        Ok(())
    }
}
