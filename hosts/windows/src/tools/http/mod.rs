use std::time::Duration;

use operit_host_api::{HostError, HostResult, HttpHost, HttpRequestData, HttpResponseData};
use reqwest::blocking::{multipart, Client};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Proxy};

#[derive(Clone, Debug, Default)]
pub struct WindowsHttpHost;

impl WindowsHttpHost {
    pub fn new() -> Self {
        Self
    }
}

impl HttpHost for WindowsHttpHost {
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        std::thread::spawn(move || executeHttpRequestOnBlockingThread(request))
            .join()
            .map_err(|_| HostError::new("windows HTTP request thread panicked"))?
    }
}

fn executeHttpRequestOnBlockingThread(request: HttpRequestData) -> HostResult<HttpResponseData> {
        let method = Method::from_bytes(request.method.as_bytes())
            .map_err(|error| HostError::new(error.to_string()))?;
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(request.connectTimeoutSeconds))
            .timeout(Duration::from_secs(request.readTimeoutSeconds))
            .danger_accept_invalid_certs(request.ignoreSsl);
        if !request.followRedirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }
        if !request.proxyHost.trim().is_empty() && request.proxyPort > 0 {
            let proxyUrl = format!("http://{}:{}", request.proxyHost.trim(), request.proxyPort);
            builder = builder.proxy(Proxy::http(&proxyUrl).map_err(|error| HostError::new(error.to_string()))?);
        }
        let client = builder.build().map_err(|error| HostError::new(error.to_string()))?;
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
        let response = httpRequest.send().map_err(|error| HostError::new(error.to_string()))?;
        let finalUrl = response.url().to_string();
        let status = response.status();
        let statusCode = status.as_u16() as i32;
        let statusMessage = status.canonical_reason().unwrap_or("").to_string();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
            .collect::<Vec<_>>();
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
