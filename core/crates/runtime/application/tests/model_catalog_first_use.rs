#![allow(non_snake_case)]

#[path = "../src/services/ModelCatalogService.rs"]
mod catalog;

// An independent process state reads the same persisted runtime storage after an update.
#[path = "../src/services/ModelCatalogService.rs"]
mod restartedCatalog;

#[path = "../src/data/preferences/ModelSpecResolver.rs"]
mod resolver;

mod services {
    pub(crate) use crate::catalog as ModelCatalogService;
}

mod ModelConfigManager {
    pub use operit_runtime::data::preferences::ModelConfigManager::ModelConfigError;
}

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::time::Duration;

use operit_host_api::*;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ModelConfigData::{ApiProviderType, ModelProfile, ProviderProfile};

const PROVIDERS: &[u8] = br#"{
    "openai": {"models": {"gpt-5": {
        "id": "gpt-5", "name": "GPT-5", "tool_call": true,
        "modalities": {"input": ["text", "image"], "output": ["text"]},
        "limit": {"context": 400000, "output": 128000}
    }}}
}"#;

struct Response {
    url: &'static str,
    body: Result<Vec<u8>, String>,
    gate: Option<(mpsc::Sender<()>, Arc<Barrier>)>,
}

#[derive(Default)]
struct CatalogHttpHost {
    responses: Mutex<VecDeque<Response>>,
    requests: Mutex<Vec<String>>,
}

impl CatalogHttpHost {
    /// Supplies the next exact catalog response without accessing a real network.
    fn respond(&self, file: &'static str, body: Result<Vec<u8>, String>) {
        self.responses.lock().unwrap().push_back(Response {
            url: file,
            body,
            gate: None,
        });
    }
}

impl HttpStreamHost for CatalogHttpHost {
    /// Rejects streaming because catalog requests use buffered HTTP.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("catalog must use buffered HTTP")
    }

    /// Rejects stream cancellation because no stream can be opened.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        panic!("catalog must use buffered HTTP")
    }
}

impl HttpHost for CatalogHttpHost {
    /// Declares host-owned byte delivery.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }

    /// Executes a scripted response, optionally pausing the first download in flight.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        self.requests.lock().unwrap().push(request.url.clone());
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected request");
        assert_eq!(request.url, format!("https://models.dev/{}", response.url));
        if let Some((entered, release)) = response.gate {
            entered.send(()).unwrap();
            release.wait();
        }
        Ok(HttpResponseData {
            finalUrl: request.url,
            statusCode: 200,
            statusMessage: "OK".into(),
            headers: Vec::new(),
            body: response.body.map_err(HostError::new)?,
        })
    }

    /// Rejects file downloads because catalog files must be parsed before storage.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("catalog must use buffered HTTP")
    }
}

/// Exercises offline first use, background replacement, retry delays and persisted precedence.
#[test]
fn bundled_catalog_is_immediate_and_downloaded_catalog_replaces_it() {
    let root =
        std::env::temp_dir().join(format!("operit-catalog-first-use-{}", std::process::id()));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost(storage.clone());
    let http = Arc::new(CatalogHttpHost::default());
    operit_host_api::HostManager::setDefaultHttpHost(http.clone());
    let cache = operit_util::RuntimeStorageLayout::RUNTIME_MODELS_DEV_CACHE_DIR_PATH;
    let local = ProviderProfile::new(
        "local".into(),
        "Local".into(),
        ApiProviderType::LOCAL_MODEL,
        String::new(),
    );
    assert!(resolver::ModelSpecResolver::new(&local).is_ok());
    assert!(http.requests.lock().unwrap().is_empty());
    let provider = ProviderProfile::new(
        "openai".into(),
        "OpenAI".into(),
        ApiProviderType::OPENAI,
        "https://api.openai.com/v1/chat/completions".into(),
    );

    // An interrupted device download cannot replace the bundled snapshot.
    storage
        .writeBytes(&format!("{cache}/api.json"), PROVIDERS)
        .unwrap();
    let bundled = catalog::requireCatalog().unwrap();
    let facts = bundled.facts(&[], "gpt-5").unwrap();
    assert!(facts.capabilities.directImage);
    assert!(facts.capabilities.toolCall);
    let resolved = resolver::ModelSpecResolver::new(&provider)
        .unwrap()
        .resolve(&ModelProfile::new("gpt-5".into()));
    assert!(resolved.capabilities.directImage);
    assert!(resolved.request.supportsStructuredTools);
    assert!(http.requests.lock().unwrap().is_empty());

    // A refused initial update is diagnostic only; resolution remains immediately usable.
    http.respond("api.json", Err("fixture offline".into()));
    let failure = catalog::refreshCatalog();
    assert!(failure.lastError.unwrap().contains("fixture offline"));
    assert!(failure.updatedAt.is_none());
    assert!(resolver::ModelSpecResolver::new(&provider).is_ok());
    assert!(Arc::ptr_eq(&bundled, &catalog::requireCatalog().unwrap()));
    assert_eq!(http.requests.lock().unwrap().len(), 1);

    // Automatic checks respect their delay; an explicit update can retry immediately.
    catalog::refreshCatalogIfStale();
    assert_eq!(http.requests.lock().unwrap().len(), 1);
    http.respond("api.json", Ok(PROVIDERS.to_vec()));
    http.respond("models.json", Ok(b"invalid json".to_vec()));
    assert!(catalog::refreshCatalog()
        .lastError
        .unwrap()
        .contains("models.json"));
    assert!(Arc::ptr_eq(&bundled, &catalog::requireCatalog().unwrap()));
    assert!(!storage.exists(&format!("{cache}/downloaded.json")).unwrap());

    // Readers keep using the bundle while a replacement is still being downloaded.
    let (entered, downloadStarted) = mpsc::channel();
    let release = Arc::new(Barrier::new(2));
    http.responses.lock().unwrap().push_back(Response {
        url: "api.json",
        body: Ok(PROVIDERS.to_vec()),
        gate: Some((entered, release.clone())),
    });
    http.respond("models.json", Ok(b"{}".to_vec()));
    let download = std::thread::spawn(catalog::refreshCatalog);
    downloadStarted
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let (completed, results) = mpsc::channel();
    let readers = (0..4)
        .map(|_| {
            let completed = completed.clone();
            std::thread::spawn(move || {
                let result = catalog::requireCatalog();
                completed.send(()).unwrap();
                result
            })
        })
        .collect::<Vec<_>>();
    for _ in 0..readers.len() {
        results.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    for reader in readers {
        assert!(Arc::ptr_eq(&bundled, &reader.join().unwrap().unwrap()));
    }
    release.wait();
    assert!(download.join().unwrap().lastError.is_none());
    let ready = catalog::requireCatalog().unwrap();
    assert!(!Arc::ptr_eq(&bundled, &ready));
    // Successful downloads replace the whole bundle, rather than merging stale entries.
    assert!(bundled.facts(&[], "claude-sonnet-4-5").is_some());
    assert!(ready.facts(&[], "claude-sonnet-4-5").is_none());
    assert_eq!(http.requests.lock().unwrap().len(), 5);
    let facts = ready.facts(&[], "gpt-5").unwrap();
    assert!(facts.capabilities.directImage);
    assert!(facts.capabilities.toolCall);
    let resolved = resolver::ModelSpecResolver::new(&provider)
        .unwrap()
        .resolve(&ModelProfile::new("gpt-5".into()));
    assert!(resolved.capabilities.directImage);
    assert!(resolved.capabilities.toolCall);
    assert!(resolved.request.supportsStructuredTools);
    assert!(storage.exists(&format!("{cache}/downloaded.json")).unwrap());
    let status = catalog::catalogStatus();
    assert!(status.updatedAt.is_some());
    assert!(status.lastError.is_none());

    // A new runtime reads the completed download instead of the embedded snapshot.
    let persisted = restartedCatalog::requireCatalog().unwrap();
    assert!(persisted.facts(&[], "claude-sonnet-4-5").is_none());
    assert_eq!(
        restartedCatalog::catalogStatus().updatedAt,
        status.updatedAt
    );
    assert_eq!(http.requests.lock().unwrap().len(), 5);

    // A later update failure preserves the complete catalog and its download time.
    http.respond("api.json", Err("fixture update failed".into()));
    let failed = catalog::refreshCatalog();
    assert_eq!(failed.updatedAt, status.updatedAt);
    assert!(failed.lastError.unwrap().contains("fixture update failed"));
    assert!(Arc::ptr_eq(&ready, &catalog::requireCatalog().unwrap()));
    assert_eq!(http.requests.lock().unwrap().len(), 6);
    assert!(http.responses.lock().unwrap().is_empty());

    std::fs::remove_dir_all(root).unwrap();
}
