//! Exercises the same catalog fetcher used by the provider settings page.
#![allow(non_snake_case)]

use operit_host_api::HostManager::setDefaultHttpHost;
use operit_host_api::{
    HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback, HttpDownloadRequest,
    HttpDownloadResult, HttpHost, HttpImageDelivery, HttpRequestData, HttpResponseData,
    HttpStreamChunkCallback, HttpStreamClosedCallback, HttpStreamHost, HttpStreamOpenedCallback,
};
use operit_model::ModelCatalog::ModelCatalog;
use operit_model::ModelConfigData::{
    ApiProviderType, AvailableProviderModelSource, ProviderProfile,
};
use operit_providers::chat::llmprovider::ModelListFetcher::ModelListFetcher;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct CatalogHttpHost;

impl HttpStreamHost for CatalogHttpHost {
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        Err(HostError::new("catalog tests do not use byte streams"))
    }

    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        Err(HostError::new("catalog tests do not use byte streams"))
    }
}

impl HttpHost for CatalogHttpHost {
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| HostError::new(e.to_string()))?;
        let mut builder = client.get(&request.url);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        let response = builder.send().map_err(|e| HostError::new(e.to_string()))?;
        let status = response.status();
        let finalUrl = response.url().to_string();
        let body = response
            .bytes()
            .map_err(|e| HostError::new(e.to_string()))?
            .to_vec();
        Ok(HttpResponseData {
            finalUrl,
            statusCode: i32::from(status.as_u16()),
            statusMessage: status.canonical_reason().unwrap_or_default().to_string(),
            headers: Vec::new(),
            body,
        })
    }

    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        Err(HostError::new("catalog tests do not download files"))
    }
}

#[test]
fn settings_fetcher_preserves_opencode_routes_headers_and_model_ids() {
    setDefaultHttpHost(Arc::new(CatalogHttpHost));
    let catalog = ModelCatalog::provider("OPENCODE").unwrap();
    for (basePath, expectedPath, apiKey) in [
        ("/zen", "/zen/v1/models", ""),
        ("/zen/go", "/zen/go/v1/models", "sk-test"),
        (
            "/proxy/zen/go/v1/",
            "/proxy/zen/go/v1/models",
            "Bearer sk-test",
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}{basePath}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "fetcher never sent an HTTP request"
                        );
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("catalog test server failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 2048];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0, "client closed before completing HTTP headers");
                request.extend_from_slice(&buf[..n]);
            }
            let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
            let body = r#"{"object":"list","data":[{"id":"big-pickle"},{"id":"kimi-k2.5"}]}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            request
        });
        let mut provider = ProviderProfile::new(
            "opencode-test".to_string(),
            "OpenCode".to_string(),
            ApiProviderType::OPENCODE,
            endpoint,
        );
        provider.apiKey = apiKey.to_string();
        provider.customHeaders =
            r#"{"User-Agent":"wrong-client","X-Custom":"kept","Authorization":""}"#.to_string();
        let result = ModelListFetcher::fetch(&provider, &catalog);
        let request = server.join().unwrap();
        let models = result.unwrap();
        assert!(
            request.starts_with(&format!("get {expectedPath} http/1.1\r\n")),
            "{request}"
        );
        assert!(request.contains(&format!(
            "user-agent: operit/{}\r\n",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(!request.contains("wrong-client"));
        assert!(request.contains("x-custom: kept\r\n"));
        if basePath.ends_with("/zen") {
            assert!(!request.contains("x-opencode-session:"));
        } else {
            assert!(request.contains("x-opencode-session: operit-opencode-test\r\n"));
        }
        if apiKey.is_empty() {
            assert!(!request.contains("authorization:"));
        } else {
            assert!(request.contains("authorization: bearer sk-test\r\n"));
        }
        assert_eq!(
            models
                .iter()
                .map(|model| model.modelId.as_str())
                .collect::<Vec<_>>(),
            vec!["big-pickle", "kimi-k2.5"]
        );
        assert!(models
            .iter()
            .all(|model| model.source == AvailableProviderModelSource::Remote));
    }
}

#[test]
#[ignore = "queries the public OpenCode Zen and Go model catalogs"]
fn live_opencode_zen_and_go_catalogs_are_fetchable_without_api_keys() {
    setDefaultHttpHost(Arc::new(CatalogHttpHost));
    let catalog = ModelCatalog::provider("OPENCODE").unwrap();
    for endpoint in ["https://opencode.ai/zen", "https://opencode.ai/zen/go"] {
        let provider = ProviderProfile::new(
            "opencode-live-test".to_string(),
            "OpenCode".to_string(),
            ApiProviderType::OPENCODE,
            endpoint.to_string(),
        );
        let models = ModelListFetcher::fetch(&provider, &catalog).unwrap();
        assert!(!models.is_empty(), "{endpoint} should list models");
        println!(
            "{endpoint}: {} models; first = {}",
            models.len(),
            models[0].modelId
        );
    }
}
