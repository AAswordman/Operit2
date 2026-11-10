#![allow(non_snake_case)]
//! Keeps this device's copy of the models.dev catalog fresh.
//!
//! The catalog is downloaded from models.dev into node-local runtime storage and read from
//! there on first use. A copy older than a day is downloaded again: when the user fetches a
//! provider's models, and on an hourly check for as long as the process runs. A failed
//! download keeps the copy in use and is reported through the catalog status; it is not
//! retried for half an hour unless the user asks.
//! Capability resolution requires a complete catalog. First use waits for an in-flight
//! download or starts one, and reports failure instead of resolving against an empty catalog.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, defaultHttpHost};
use operit_host_api::HttpRequestData;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_model::ModelConfigData::ModelCatalogStatus;
use operit_model::ModelsDevCatalog::ModelsDevCatalog;
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::RUNTIME_MODELS_DEV_CACHE_DIR_PATH;

const PROVIDER_CATALOG_URL: &str = "https://models.dev/api.json";
const LAB_CATALOG_URL: &str = "https://models.dev/models.json";
const PROVIDER_CATALOG_FILE: &str = "api.json";
const LAB_CATALOG_FILE: &str = "models.json";
/// Written last, so a catalog without it is an interrupted download.
const STAMP_FILE: &str = "downloaded.json";
/// A catalog older than this is downloaded again.
const STALE_AFTER_MS: i64 = 24 * 60 * 60 * 1000;
/// A failed download is not tried again on its own sooner than this.
const RETRY_AFTER_MS: i64 = 30 * 60 * 1000;
/// How often a running process looks at the catalog's age.
const CHECK_EVERY_MS: u64 = 60 * 60 * 1000;
const LOG_TAG: &str = "ModelCatalog";

/// The catalog in use and what the last download did.
#[derive(Default)]
struct CatalogState {
    loaded: bool,
    catalog: Option<Arc<ModelsDevCatalog>>,
    updatedAt: Option<i64>,
    lastError: Option<String>,
    lastFailureAt: Option<i64>,
    downloadAttempts: u64,
}

static STATE: OnceLock<Mutex<CatalogState>> = OnceLock::new();
/// Serializes downloads, so a background check and a fetch never download twice.
static DOWNLOAD: Mutex<()> = Mutex::new(());
static KEEPING_FRESH: AtomicBool = AtomicBool::new(false);

/// Returns the process-wide catalog state.
fn state() -> &'static Mutex<CatalogState> {
    STATE.get_or_init(|| Mutex::new(CatalogState::default()))
}

/// Returns the catalog downloaded on this device, read from storage on first use.
pub(crate) fn currentCatalog() -> Option<Arc<ModelsDevCatalog>> {
    let mut state = state().lock().expect("model catalog state lock poisoned");
    if !state.loaded {
        state.loaded = true;
        loadStoredCatalog(&mut state);
    }
    state.catalog.clone()
}

/// Waits for the first complete catalog, sharing an in-flight download with other callers.
/// A new call can retry a failed first download without the background retry delay.
pub(crate) fn requireCatalog() -> Result<Arc<ModelsDevCatalog>, String> {
    currentCatalog();
    let attempts = {
        let state = state().lock().expect("model catalog state lock poisoned");
        if let Some(catalog) = &state.catalog {
            return Ok(catalog.clone());
        }
        state.downloadAttempts
    };
    let _download = DOWNLOAD.lock().expect("model catalog download lock poisoned");
    {
        let state = state().lock().expect("model catalog state lock poisoned");
        if state.catalog.is_some() || state.downloadAttempts != attempts {
            return requiredCatalogResult(&state);
        }
    }
    downloadCatalog();
    requiredCatalogResult(&state().lock().expect("model catalog state lock poisoned"))
}

/// Reports an unavailable catalog instead of interpreting missing facts as disabled capabilities.
fn requiredCatalogResult(state: &CatalogState) -> Result<Arc<ModelsDevCatalog>, String> {
    match &state.catalog {
        Some(catalog) => Ok(catalog.clone()),
        None => Err(format!(
            "Model capability catalog is not ready; retry after downloading models.dev: {}",
            state.lastError.as_deref().unwrap_or("no complete catalog downloaded")
        )),
    }
}

/// Reports when the catalog in use was downloaded and why the last download failed.
pub(crate) fn catalogStatus() -> ModelCatalogStatus {
    currentCatalog();
    let state = state().lock().expect("model catalog state lock poisoned");
    ModelCatalogStatus {
        updatedAt: state.updatedAt,
        lastError: state.lastError.clone(),
    }
}

/// Downloads the catalog now and reports the result.
pub(crate) fn refreshCatalog() -> ModelCatalogStatus {
    {
        let _download = DOWNLOAD.lock().expect("model catalog download lock poisoned");
        downloadCatalog();
    }
    catalogStatus()
}

/// Downloads the catalog when the copy on this device is missing or over a day old.
pub(crate) fn refreshCatalogIfStale() {
    if !needsDownload() {
        return;
    }
    let _download = DOWNLOAD.lock().expect("model catalog download lock poisoned");
    // Another caller may have downloaded it while this one waited.
    if needsDownload() {
        downloadCatalog();
    }
}

/// Looks at the catalog's age now and every hour after, for as long as the process runs.
pub(crate) fn startKeepingFresh() {
    if KEEPING_FRESH.swap(true, Ordering::SeqCst) {
        return;
    }
    let scheduled = defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeTask("model-catalog-refresh", Box::new(checkAndReschedule));
    if let Err(error) = scheduled {
        AppLogger::e(LOG_TAG, &format!("model catalog check not scheduled: {error}"));
    }
}

/// Downloads a stale catalog, then schedules the next look.
fn checkAndReschedule() {
    refreshCatalogIfStale();
    let scheduled = defaultHostRuntimeTaskSchedulerHost().scheduleDelayedHostRuntimeTask(
        "model-catalog-refresh",
        CHECK_EVERY_MS,
        Box::new(checkAndReschedule),
    );
    if let Err(error) = scheduled {
        AppLogger::e(LOG_TAG, &format!("model catalog check not scheduled: {error}"));
    }
}

/// Returns whether the catalog is missing or stale and no download failed in the last half hour.
fn needsDownload() -> bool {
    currentCatalog();
    let state = state().lock().expect("model catalog state lock poisoned");
    let now = currentTimeMillis();
    if state
        .lastFailureAt
        .is_some_and(|failedAt| now - failedAt < RETRY_AFTER_MS)
    {
        return false;
    }
    state
        .updatedAt
        .map_or(true, |updatedAt| now - updatedAt > STALE_AFTER_MS)
}

/// Downloads both catalogs, stores them and puts them in use; a failure keeps the copy in use.
fn downloadCatalog() {
    currentCatalog();
    let downloaded = fetchCatalog().and_then(|(catalog, providerCatalog, labCatalog)| {
        let updatedAt = currentTimeMillis();
        storeCatalog(&providerCatalog, &labCatalog, updatedAt)?;
        Ok((catalog, updatedAt))
    });
    let mut state = state().lock().expect("model catalog state lock poisoned");
    state.downloadAttempts += 1;
    match downloaded {
        Ok((catalog, updatedAt)) => {
            state.catalog = Some(Arc::new(catalog));
            state.updatedAt = Some(updatedAt);
            state.lastError = None;
            state.lastFailureAt = None;
            AppLogger::i(LOG_TAG, "models.dev catalog downloaded");
        }
        Err(error) => {
            AppLogger::w(LOG_TAG, &format!("models.dev catalog download failed: {error}"));
            state.lastError = Some(error);
            state.lastFailureAt = Some(currentTimeMillis());
        }
    }
}

/// Downloads and indexes both catalogs, returning the index and the bytes to store.
fn fetchCatalog() -> Result<(ModelsDevCatalog, Vec<u8>, Vec<u8>), String> {
    let providerCatalog = download(PROVIDER_CATALOG_URL)?;
    let labCatalog = download(LAB_CATALOG_URL)?;
    let catalog = ModelsDevCatalog::parse(&providerCatalog, &labCatalog)?;
    Ok((catalog, providerCatalog, labCatalog))
}

/// Loads one catalog file through the runtime HTTP host.
fn download(url: &str) -> Result<Vec<u8>, String> {
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url: url.to_string(),
            method: "GET".to_string(),
            headers: vec![("User-Agent".to_string(), "Operit".to_string())],
            body: Vec::new(),
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 15,
            readTimeoutSeconds: 60,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .map_err(|error| format!("{url}: {error}"))?;
    if !(200..300).contains(&response.statusCode) {
        return Err(format!("{url}: HTTP {}", response.statusCode));
    }
    Ok(response.body)
}

/// Stores both catalogs and then the stamp that dates them.
fn storeCatalog(providerCatalog: &[u8], labCatalog: &[u8], updatedAt: i64) -> Result<(), String> {
    let storage = defaultRuntimeStorageHost();
    let stampPath = cachePath(STAMP_FILE);
    if storage.exists(&stampPath).map_err(|error| error.message)? {
        storage
            .delete(&stampPath, false)
            .map_err(|error| error.message)?;
    }
    storage
        .writeBytes(&cachePath(PROVIDER_CATALOG_FILE), providerCatalog)
        .map_err(|error| error.message)?;
    storage
        .writeBytes(&cachePath(LAB_CATALOG_FILE), labCatalog)
        .map_err(|error| error.message)?;
    let stamp = serde_json::to_vec(&serde_json::json!({ "updatedAt": updatedAt }))
        .map_err(|error| error.to_string())?;
    storage
        .writeBytes(&stampPath, &stamp)
        .map_err(|error| error.message)
}

/// Reads the stored catalog into the state; a catalog never downloaded leaves it empty.
fn loadStoredCatalog(state: &mut CatalogState) {
    let storage = defaultRuntimeStorageHost();
    let stampPath = cachePath(STAMP_FILE);
    let stored = storage
        .exists(&stampPath)
        .map_err(|error| error.message)
        .and_then(|exists| {
            if !exists {
                return Ok(None);
            }
            let stamp: serde_json::Value = serde_json::from_slice(
                &storage.readBytes(&stampPath).map_err(|error| error.message)?,
            )
            .map_err(|error| format!("{STAMP_FILE}: {error}"))?;
            let updatedAt = stamp
                .get("updatedAt")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| format!("{STAMP_FILE} has no download time"))?;
            let providerCatalog = storage
                .readBytes(&cachePath(PROVIDER_CATALOG_FILE))
                .map_err(|error| error.message)?;
            let labCatalog = storage
                .readBytes(&cachePath(LAB_CATALOG_FILE))
                .map_err(|error| error.message)?;
            let catalog = ModelsDevCatalog::parse(&providerCatalog, &labCatalog)?;
            Ok(Some((catalog, updatedAt)))
        });
    match stored {
        Ok(Some((catalog, updatedAt))) => {
            state.catalog = Some(Arc::new(catalog));
            state.updatedAt = Some(updatedAt);
        }
        Ok(None) => {}
        Err(error) => {
            AppLogger::w(LOG_TAG, &format!("stored models.dev catalog unreadable: {error}"));
            state.lastError = Some(error);
        }
    }
}

/// Addresses a catalog file within node-local runtime storage.
fn cachePath(fileName: &str) -> String {
    format!("{RUNTIME_MODELS_DEV_CACHE_DIR_PATH}/{fileName}")
}
