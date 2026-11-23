use base64::Engine;
use operit_host_api::RuntimeStorageHost;
use operit_host_api::{FileSource, HostManager::HostManager, HttpDownloadControl};
use operit_util::GithubReleaseUtil::ReleaseInfo;
use operit_util::IncrementalUpdate::*;
use std::sync::Arc;

#[path = "incremental_updates/chains.rs"]
mod chains;
#[path = "incremental_updates/batching.rs"]
mod batching;
#[path = "incremental_updates/progress.rs"]
mod preparation_progress;

#[path = "../../../../../hosts/android/src/update_apk.rs"]
mod update_apk;
#[path = "../../../../../hosts/android/src/update_base.rs"]
mod update_base;
#[path = "../../../../../hosts/android/src/system_operation.rs"]
mod android_system_operation;
#[path = "../../../../../apps/flutter/native/operit-flutter-bridge/src/FlutterSystemOperationBridge.rs"]
mod flutter_system_operation;
#[path = "incremental_updates/apk.rs"]
mod apk;

#[cfg(windows)]
#[path = "../../../../../hosts/windows/src/process.rs"]
mod process;
#[cfg(windows)]
#[path = "../../../../../hosts/windows/src/tools/system/update_msi.rs"]
mod update_msi;
#[cfg(windows)]
#[path = "incremental_updates/msi.rs"]
mod msi;

/// Removes only the retained original while leaving authenticated patch metadata and bytes available.
fn removeRetainedBase(fixture: &Fixture, release: &ReleaseInfo, current: &str) {
    let marker = format!("-{}.", release.version);
    let (family, extension) = release.assetName.split_once(&marker).unwrap();
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .delete(
            &format!("runtime/application_update_bases/{family}.{extension}/{current}"),
            false,
        )
        .unwrap();
}

/// Discovery combines both repositories and downloads patches from their authenticated source.
#[tokio::test]
async fn discovery_selects_patch_only_newer_release_and_authenticates_its_manifest() {
    use operit_util::GithubReleaseUtil::{
        FullUpdateChannel, FullUpdateStatus, FullUpdateTarget, GithubReleaseUtil,
    };
    let mut fixture = Fixture::new();
    let (release, original_http, current) = fixture.network(true, false);
    let manifest = release.updateManifest.as_ref().unwrap();
    let patch_url = manifest.url.replace("/Operit2/", "/Operit2-nightly/");
    let metadata = serde_json::json!([{
        "tag_name": format!("v{}", release.version), "body": "patch only",
        "html_url": "patches", "draft": false, "prerelease": true,
        "assets": [
            {"name": manifest.name, "browser_download_url": patch_url, "digest": manifest.digest},
            {"name": release.patchAssetNames[0], "browser_download_url": "unused", "digest": null}
        ]
    }]);
    let full_metadata = serde_json::json!([{
        "tag_name": format!("v{}", release.version), "body": "full installer",
        "html_url": "source", "draft": false, "prerelease": true,
        "assets": [{"name": release.assetName, "browser_download_url": release.downloadUrl,
                    "digest": release.packageSha256}]
    }]);
    let mut bodies: std::collections::BTreeMap<_, _> = original_http
        .bodies
        .iter()
        .map(|(url, body)| {
            let url = if url.ends_with(".update.json") || url.ends_with(".patch.gz") {
                url.replace("/Operit2/", "/Operit2-nightly/")
            } else {
                url.clone()
            };
            (url, body.clone())
        })
        .collect();
    for (repo, data) in [("Operit2", full_metadata), ("Operit2-nightly", metadata)] {
        bodies.insert(
            format!("https://api.github.com/repos/AAswordman/{repo}/releases?page=1&per_page=30"),
            serde_json::to_vec(&data).unwrap(),
        );
    }
    let http = Arc::new(HttpFixture {
        bodies,
        requests: std::sync::Mutex::new(Vec::new()),
        failedUrl: std::sync::Mutex::new(None),
        statuses: std::sync::Mutex::new(Default::default()),
    });
    fixture.context = fixture.context.clone().withHttpHost(http.clone());
    operit_host_api::HostManager::setDefaultHttpHost(http.clone());
    let discovered = match GithubReleaseUtil::checkForFullUpdate(
        &fixture.manifest.patches[0].baseVersion,
        FullUpdateTarget::app("android", "arm64-v8a").unwrap(),
    )
    .await
    .unwrap()
    {
        FullUpdateStatus::Available(info) => info,
        FullUpdateStatus::UpToDate => panic!("patch-only update was hidden"),
    };
    assert_eq!(discovered.version, release.version);
    assert!(discovered.fullPackageAvailable);
    let patch_only = GithubReleaseUtil::fetchLatestReleaseInfo(
        "AAswordman",
        "Operit2-nightly",
        FullUpdateTarget::app("android", "arm64-v8a").unwrap(),
        FullUpdateChannel::Preview,
    )
    .await
    .unwrap();
    assert!(!patch_only.fullPackageAvailable);
    assert_eq!(patch_only.version, release.version);
    let plan = planUpdate(
        discovered,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.mode, UpdateMode::Incremental);
    assert!(plan.fullAvailable);
    let path = executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
        .await
        .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[3..]
        .iter()
        .all(|url| url.contains("/Operit2-nightly/")));
}

/// Authenticated metadata, full packages, and patches remain usable when only a mirror is reachable.
#[tokio::test]
async fn manifest_and_both_package_routes_work_with_only_a_reachable_mirror() {
    for retained in [false, true] {
        let mut fixture = Fixture::new();
        let (release, original, current) = fixture.network(retained, false);
        let http = Arc::new(HttpFixture {
            bodies: original
                .bodies
                .iter()
                .map(|(url, bytes)| (format!("https://ghfast.top/{url}"), bytes.clone()))
                .collect(),
            requests: std::sync::Mutex::new(Vec::new()),
            failedUrl: std::sync::Mutex::new(None),
            statuses: std::sync::Mutex::new(Default::default()),
        });
        fixture.context = fixture.context.clone().withHttpHost(http.clone());
        assert_eq!(
            fixture.download(release, current).await.unwrap(),
            fixture.target
        );
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|url| url.starts_with("https://ghfast.top/https://github.com/")));
        assert!(requests[0].ends_with(".update.json"));
        assert_eq!(requests[1].ends_with(".patch.gz"), retained);
    }
}

/// Planning never transfers package bytes, and a rejected original cannot poison the next selection.
#[tokio::test]
async fn selected_base_is_verified_before_patch_download_for_patch_only_release() {
    let mut fixture = Fixture::new();
    let (mut release, http, current) = fixture.network(true, false);
    removeRetainedBase(&fixture, &release, &current);
    release.fullPackageAvailable = false;
    release.packageSha256 = None;
    let control = HttpDownloadControl::new();
    let mut plan = planUpdate(release, current, &fixture.context, control.clone())
        .await
        .unwrap();
    assert_eq!(plan.mode, UpdateMode::NeedsBase);
    assert!(!plan.fullAvailable);
    assert!(plan.incrementalAvailable);
    assert_eq!(plan.baseVersion.as_deref(), Some("2.0.0-preview.1"));
    assert_eq!(http.requests.lock().unwrap().len(), 1);
    let wrong = fixture.selected("wrong.zip", b"PK\x03\x04wrong original installer");
    assert!(plan
        .supplyBase(&wrong, &fixture.context, &control)
        .unwrap_err()
        .starts_with("UPDATE_BASE_MISMATCH"));
    assert_eq!(plan.mode, UpdateMode::NeedsBase);
    assert_eq!(http.requests.lock().unwrap().len(), 1);
    let selected = fixture.selected("renamed-original.apk", &fixture.base.readBytes().unwrap());
    plan.supplyBase(&selected, &fixture.context, &control)
        .unwrap();
    assert_eq!(plan.mode, UpdateMode::Incremental);
    let path = executeUpdate(plan, &fixture.context, control, |_| {})
        .await
        .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.last().unwrap().ends_with(".patch.gz"));
}

/// A different cached build of the same version must request the authenticated original.
#[tokio::test]
async fn mismatched_cached_build_requests_original_without_disabling_incremental() {
    for fullAvailable in [false, true] {
        let mut fixture = Fixture::new();
        let (mut release, http, current) = fixture.network(true, false);
        release.fullPackageAvailable = fullAvailable;
        let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
        let family = release
            .assetName
            .split_once(&format!("-{}.", release.version))
            .unwrap()
            .0;
        storage
            .writeBytes(
                &format!("runtime/application_update_bases/{family}.apk/{current}"),
                b"PK\x03\x04earlier build with the same preview number",
            )
            .unwrap();
        let control = HttpDownloadControl::new();
        let mut plan = planUpdate(
            release.clone(),
            current.clone(),
            &fixture.context,
            control.clone(),
        )
        .await
        .unwrap();
        assert_eq!(plan.mode, UpdateMode::NeedsBase);
        assert!(plan.incrementalAvailable);
        assert_eq!(http.requests.lock().unwrap().len(), 1);
        assert!(!storage
            .exists(&format!(
                "runtime/application_update_state/{family}.apk.json"
            ))
            .unwrap());
        let selected = fixture.selected("original.apk", &fixture.base.readBytes().unwrap());
        plan.supplyBase(&selected, &fixture.context, &control)
            .unwrap();
        let path = executeUpdate(plan, &fixture.context, control, |_| {})
            .await
            .unwrap();
        assert_eq!(storage.readBytes(&path).unwrap(), fixture.target);
        assert_eq!(http.requests.lock().unwrap().len(), 2);
    }
}

/// Old full-only state cannot hide a patch-only release or its original-installer picker.
#[tokio::test]
async fn patch_only_release_ignores_persisted_full_only_policy() {
    let mut fixture = Fixture::new();
    let (mut release, http, current) = fixture.network(true, false);
    release.fullPackageAvailable = false;
    removeRetainedBase(&fixture, &release, &current);
    let family = release
        .assetName
        .split_once(&format!("-{}.", release.version))
        .unwrap()
        .0;
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .writeBytes(
            &format!("runtime/application_update_state/{family}.apk.json"),
            &serde_json::to_vec(&serde_json::json!({
                "version": release.version, "assetName": release.assetName,
            }))
            .unwrap(),
        )
        .unwrap();
    fixture.restart();
    fixture.context = fixture
        .context
        .clone()
        .withHttpHost(http.clone())
        .withHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
        ));
    let mut plan = planUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.mode, UpdateMode::NeedsBase);
    assert!(plan.incrementalAvailable);
    let selected = fixture.selected("original.apk", &fixture.base.readBytes().unwrap());
    plan.supplyBase(&selected, &fixture.context, &HttpDownloadControl::new())
        .unwrap();
    let path = executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
        .await
        .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
}

/// A real patch failure is reported without choosing a nonexistent full package on retry.
#[tokio::test]
async fn patch_only_failure_does_not_persist_an_unavailable_full_route() {
    let mut fixture = Fixture::new();
    let (mut release, _, current) = fixture.network(true, true);
    release.fullPackageAvailable = false;
    assert!(fixture.download(release, current).await.is_err());
    fixture.restart();
    let (mut release, http, current) = fixture.network(true, false);
    release.fullPackageAvailable = false;
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(http.requests.lock().unwrap().len(), 2);
    assert!(http.requests.lock().unwrap()[1].ends_with(".patch.gz"));
}

/// Declining the original downloads a full asset only when the same plan confirms one exists.
#[tokio::test]
async fn no_original_choice_uses_full_or_stops_without_requesting_a_nonexistent_full_asset() {
    for fullAvailable in [false, true] {
        let mut fixture = Fixture::new();
        let (mut release, http, current) = fixture.network(true, false);
        removeRetainedBase(&fixture, &release, &current);
        release.fullPackageAvailable = fullAvailable;
        let fullUrl = release.downloadUrl.clone();
        let control = HttpDownloadControl::new();
        let mut plan = planUpdate(release, current, &fixture.context, control.clone())
            .await
            .unwrap();
        assert_eq!(plan.mode, UpdateMode::NeedsBase);
        plan.chooseFull();
        let result = executeUpdate(plan, &fixture.context, control, |_| {}).await;
        if fullAvailable {
            assert!(result.is_ok());
            assert_eq!(http.requests.lock().unwrap().last(), Some(&fullUrl));
        } else {
            assert_eq!(result.unwrap_err(), "UPDATE_UNAVAILABLE");
            assert_eq!(http.requests.lock().unwrap().len(), 1);
        }
    }
}

/// Missing assets, inapplicable base versions and oversized deltas cannot become selectable routes.
#[tokio::test]
async fn planner_counts_only_published_applicable_smaller_patches() {
    for fullAvailable in [false, true] {
        for case in 0..3 {
            let mut fixture = Fixture::new();
            if case == 1 {
                fixture.manifest.patches[0].size = fixture.manifest.size;
            }
            let (mut release, http, mut current) = fixture.network(true, false);
            release.fullPackageAvailable = fullAvailable;
            if case == 0 {
                release.patchAssetNames.clear();
            }
            if case == 2 {
                current = current.replace("preview.1", "preview.3");
            }
            let plan = planUpdate(
                release,
                current,
                &fixture.context,
                HttpDownloadControl::new(),
            )
            .await
            .unwrap();
            let patchOnlyOversized = case == 1 && !fullAvailable;
            assert_eq!(plan.incrementalAvailable, patchOnlyOversized);
            assert_eq!(
                plan.mode,
                if patchOnlyOversized {
                    UpdateMode::Incremental
                } else if fullAvailable {
                    UpdateMode::Full
                } else {
                    UpdateMode::Unavailable
                }
            );
            assert_eq!(http.requests.lock().unwrap().len(), 1);
        }
    }
}

/// A patch removed after planning reports unavailability without trying a full URL absent from the release.
#[tokio::test]
async fn patch_only_asset_disappearance_never_downloads_full() {
    for manifestMissing in [false, true] {
        let mut fixture = Fixture::new();
        let (mut release, http, current) = fixture.network(true, false);
        release.fullPackageAvailable = false;
        release.packageSha256 = None;
        let missing = if manifestMissing {
            release.updateManifest.as_ref().unwrap().url.clone()
        } else {
            http.bodies
                .keys()
                .find(|url| url.ends_with(".patch.gz"))
                .unwrap()
                .clone()
        };
        http.statuses.lock().unwrap().insert(missing, 404);
        let fullUrl = release.downloadUrl.clone();
        let control = HttpDownloadControl::new();
        let plan = planUpdate(release, current, &fixture.context, control.clone())
            .await
            .unwrap();
        let error = executeUpdate(plan, &fixture.context, control, |_| {})
            .await
            .unwrap_err();
        assert!(error.starts_with("UPDATE_UNAVAILABLE"));
        assert!(!http
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|url| url == &fullUrl));
    }
}

struct Fixture {
    root: std::path::PathBuf,
    context: HostManager,
    manifest: UpdateManifest,
    base: operit_vfs::FileReference::FileReference,
    patch: operit_vfs::FileReference::FileReference,
    target: Vec<u8>,
}

impl Fixture {
    /// Loads a patch generated by the real Python publisher, then uses native Host VFS adapters.
    fn new() -> Self {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/package_delta.json")).unwrap();
        let manifest: UpdateManifest = serde_json::from_value(data["manifest"].clone()).unwrap();
        let root = std::env::temp_dir().join(format!("operit-delta-{}", uuid::Uuid::new_v4()));
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            root.clone(),
            root.join("workspaces"),
        ));
        let context = HostManager::withFileSystemHost(Arc::new(
            operit_host_native_filesystem::PosixFileSystemHost::new(),
        ))
        .withRuntimeStorageHost(storage.clone())
        .withRuntimeStorageWriteHost(storage.clone());
        let mut base = b"PK\x03\x04".to_vec();
        base.extend(b"stable\x01\x00".repeat(10_000));
        let mut target = base.clone();
        target.extend_from_slice(b"new signed package tail");
        let patch = base64::engine::general_purpose::STANDARD
            .decode(data["patch"].as_str().unwrap())
            .unwrap();
        storage
            .writeBytes("runtime/temp/application_updates/test/base.apk", &base)
            .unwrap();
        storage
            .writeBytes(
                "runtime/temp/application_updates/test/delta.patch.gz",
                &patch,
            )
            .unwrap();
        let vfs = context.vfs().unwrap();
        let base = vfs
            .file("/operitapp/data/temp/application_updates/test/base.apk")
            .unwrap();
        let patch = vfs
            .file("/operitapp/data/temp/application_updates/test/delta.patch.gz")
            .unwrap();
        Self {
            root,
            context,
            manifest,
            base,
            patch,
            target,
        }
    }
    /// Applies the fixture through the production reconstruction entry point.
    fn apply(&self, control: &HttpDownloadControl) -> Result<(), String> {
        applyPatch(
            &self.base,
            &self.patch,
            &self.manifest,
            &self.manifest.patches[0],
            &self.context,
            "runtime/temp/application_updates/test/output.apk",
            control,
        )
    }
    /// Reads a committed result without bypassing the runtime storage boundary.
    fn output(&self) -> Vec<u8> {
        self.context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes("runtime/temp/application_updates/test/output.apk")
            .unwrap()
    }
}
impl Drop for Fixture {
    /// Removes only the unique fixture directory owned by this test.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Python generation and Rust reconstruction preserve every byte of the original package.
#[test]
fn python_patch_reconstructs_signed_package_bytes_through_vfs() {
    let fixture = Fixture::new();
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
}

/// The independent Python generator's residual instructions round-trip through the Rust decoder.
#[test]
fn python_hdiff_residuals_reconstruct_signed_package_bytes_through_vfs() {
    let mut fixture = Fixture::new();
    let data: serde_json::Value = serde_json::from_str(include_str!("fixtures/package_delta_v2.json")).unwrap();
    fixture.manifest = serde_json::from_value(data["manifest"].clone()).unwrap();
    for change in data["changes"].as_array().unwrap() {
        fixture.target[change[0].as_u64().unwrap() as usize] = change[1].as_u64().unwrap() as u8;
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(data["patch"].as_str().unwrap()).unwrap();
    fixture.context.runtimeStorageHost.as_ref().unwrap()
        .writeBytes("runtime/temp/application_updates/test/delta.patch.gz", &bytes).unwrap();
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
    fixture.manifest.schema = 3;
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
    fixture.manifest.schema = 1;
    assert!(fixture.apply(&HttpDownloadControl::new()).unwrap_err().contains("header"));
}

/// A mismatched base cannot publish any output.
#[test]
fn rejects_wrong_base_before_publication() {
    let mut fixture = Fixture::new();
    fixture.manifest.patches[0].baseSha256 = "0".repeat(64);
    assert!(fixture
        .apply(&HttpDownloadControl::new())
        .unwrap_err()
        .contains("source SHA-256"));
    assert!(!fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .exists("runtime/temp/application_updates/test/output.apk")
        .unwrap());
}

/// A target digest disagreement discards all reconstructed bytes.
#[test]
fn rejects_manifest_target_mismatch() {
    let mut fixture = Fixture::new();
    fixture.manifest.sha256 = "0".repeat(64);
    assert!(fixture.apply(&HttpDownloadControl::new()).is_err());
}

/// Cancellation during preparation cannot create an installable output.
#[test]
fn cancellation_stops_local_reconstruction() {
    let fixture = Fixture::new();
    let control = HttpDownloadControl::new();
    control.cancel();
    assert_eq!(fixture.apply(&control).unwrap_err(), "UPDATE_CANCELLED");
}

/// Corrupted compressed bytes are rejected before they reach an installer.
#[test]
fn rejects_corrupt_patch_digest() {
    let fixture = Fixture::new();
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .writeBytes(
            "runtime/temp/application_updates/test/delta.patch.gz",
            b"not a patch",
        )
        .unwrap();
    assert!(fixture
        .apply(&HttpDownloadControl::new())
        .unwrap_err()
        .contains("source SHA-256"));
}

/// The target manifest and GitHub release metadata form one authenticated identity.
#[test]
fn manifest_cannot_substitute_a_different_platform_or_digest() {
    let fixture = Fixture::new();
    let mut release = ReleaseInfo {
        version: fixture.manifest.version.clone(),
        assetName: fixture.manifest.assetName.clone(),
        downloadUrl: String::new(),
        releaseNotes: String::new(),
        releasePageUrl: String::new(),
        packageSha256: Some(format!("sha256:{}", fixture.manifest.sha256)),
        fullPackageAvailable: true,
        patchAssetNames: Vec::new(),
        updateManifest: None,
    };
    validateManifest(&fixture.manifest, &release).unwrap();
    release.assetName = "operit2-app-windows-x86_64-2.0.0-preview.2.msi".into();
    assert!(validateManifest(&fixture.manifest, &release).is_err());
}

/// Release versions retain stable selection and all existing prereleases use preview selection.
#[test]
fn release_and_preview_channels_preserve_semver_order() {
    use operit_util::GithubReleaseUtil::{FullUpdateChannel, GithubReleaseUtil};
    assert_eq!(
        GithubReleaseUtil::fullUpdateChannelForVersion("2.0.0-preview.35").unwrap(),
        FullUpdateChannel::Preview
    );
    assert_eq!(
        GithubReleaseUtil::fullUpdateChannelForVersion("2.0.0").unwrap(),
        FullUpdateChannel::Stable
    );
    assert!(GithubReleaseUtil::compareVersions("2.0.0-preview.10", "2.0.0-preview.9").unwrap() > 0);
    assert!(GithubReleaseUtil::compareVersions("2.0.0", "2.0.0-preview.35").unwrap() > 0);
}

struct HttpFixture {
    bodies: std::collections::BTreeMap<String, Vec<u8>>,
    requests: std::sync::Mutex<Vec<String>>,
    failedUrl: std::sync::Mutex<Option<String>>,
    statuses: std::sync::Mutex<std::collections::BTreeMap<String, i32>>,
}
impl operit_host_api::HttpStreamHost for HttpFixture {
    /// Rejects requests that omit the response metadata needed for authenticated downloads.
    fn openHttpByteStream(
        &self,
        _: String,
        _: operit_host_api::HttpRequestData,
        _: operit_host_api::HttpStreamOpenedCallback,
        _: operit_host_api::HttpStreamChunkCallback,
        _: operit_host_api::HttpStreamClosedCallback,
    ) -> operit_host_api::HostResult<()> {
        panic!("response headers required")
    }
    /// Delivers only the explicitly staged response so unexpected full downloads fail the test.
    fn openHttpResponseStream(
        &self,
        _: String,
        request: operit_host_api::HttpRequestData,
        head: operit_host_api::HttpStreamResponseCallback,
        bytes: operit_host_api::HttpStreamChunkCallback,
        closed: operit_host_api::HttpStreamClosedCallback,
    ) -> operit_host_api::HostResult<()> {
        // Probe samples are separate from actual artifact transfers asserted below.
        if !request
            .headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("range"))
        {
            self.requests.lock().unwrap().push(request.url.clone());
        }
        if self.failedUrl.lock().unwrap().as_ref() == Some(&request.url) {
            return Err(operit_host_api::HostError::new(
                "Simulated network interruption",
            ));
        }
        let body = self
            .bodies
            .get(&request.url)
            .ok_or_else(|| operit_host_api::HostError::new("Unexpected update transfer"))?;
        head(operit_host_api::HttpResponseHead {
            statusCode: self
                .statuses
                .lock()
                .unwrap()
                .get(&request.url)
                .copied()
                .unwrap_or(200),
            finalUrl: request.url,
            statusMessage: "OK".into(),
            headers: vec![("Content-Length".into(), body.len().to_string())],
        });
        for chunk in body.chunks(64) {
            bytes(chunk.to_vec());
        }
        closed(Ok(()));
        Ok(())
    }
    /// Releases the deterministic response without opening sockets.
    fn closeHttpByteStream(&self, _: &str) -> operit_host_api::HostResult<()> {
        Ok(())
    }
}
impl operit_host_api::HttpHost for HttpFixture {
    /// Keeps fixture responses in memory.
    fn imageDelivery(&self) -> operit_host_api::HttpImageDelivery {
        operit_host_api::HttpImageDelivery::Bytes
    }
    /// Detects accidental synchronous HTTP in the common updater.
    fn executeHttpRequest(
        &self,
        _: operit_host_api::HttpRequestData,
    ) -> operit_host_api::HostResult<operit_host_api::HttpResponseData> {
        panic!("async only")
    }
    /// Detects downloads that bypass the VFS streaming writer.
    fn downloadFiles(
        &self,
        _: operit_host_api::HttpDownloadRequest,
        _: HttpDownloadControl,
        _: operit_host_api::HttpDownloadProgressCallback,
    ) -> operit_host_api::HostResult<operit_host_api::HttpDownloadResult> {
        panic!("VFS streaming only")
    }
}

impl Fixture {
    /// Configures authenticated GitHub-style metadata and either a retained base or a full transfer.
    fn network(
        &mut self,
        retained: bool,
        corrupt: bool,
    ) -> (ReleaseInfo, Arc<HttpFixture>, String) {
        use sha2::Digest;
        let metadata = serde_json::to_vec(&self.manifest).unwrap();
        let manifestHash = format!("{:x}", sha2::Sha256::digest(&metadata));
        let asset = &self.manifest.assetName;
        let url = format!(
            "https://github.com/AAswordman/Operit2/releases/download/v{}/{asset}",
            self.manifest.version
        );
        let patchUrl = format!(
            "{}/{}",
            url.rsplit_once('/').unwrap().0,
            self.manifest.patches[0].assetName
        );
        let manifestUrl = format!("{url}.update.json");
        let mut bodies = std::collections::BTreeMap::from([
            (manifestUrl.clone(), metadata),
            (url.clone(), self.target.clone()),
        ]);
        let currentAsset = asset.replace(
            &format!("-{}.", self.manifest.version),
            &format!("-{}.", self.manifest.patches[0].baseVersion),
        );
        if retained {
            let versionSuffix = format!("-{}.", self.manifest.version);
            let (family, extension) = asset.split_once(&versionSuffix).unwrap();
            self.context
                .runtimeStorageHost
                .as_ref()
                .unwrap()
                .writeBytes(
                    &format!("runtime/application_update_bases/{family}.{extension}/{currentAsset}"),
                    &self.base.readBytes().unwrap(),
                )
                .unwrap();
            let bytes = if corrupt {
                b"\x1f\x8b\0\0broken".to_vec()
            } else {
                self.patch.readBytes().unwrap()
            };
            bodies.insert(patchUrl, bytes);
        }
        let http = Arc::new(HttpFixture {
            bodies,
            requests: std::sync::Mutex::new(Vec::new()),
            failedUrl: std::sync::Mutex::new(None),
            statuses: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        });
        self.context = self
            .context
            .clone()
            .withHttpHost(http.clone())
            .withHostRuntimeTaskSchedulerHost(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            ));
        let release = ReleaseInfo {
            version: self.manifest.version.clone(),
            assetName: asset.clone(),
            downloadUrl: url,
            releaseNotes: String::new(),
            releasePageUrl: String::new(),
            packageSha256: Some(format!("sha256:{}", self.manifest.sha256)),
            fullPackageAvailable: true,
            patchAssetNames: vec![self.manifest.patches[0].assetName.clone()],
            updateManifest: Some(operit_util::GithubReleaseUtil::ReleaseManifestAsset {
                name: format!("{asset}.update.json"),
                url: manifestUrl,
                digest: format!("sha256:{manifestHash}"),
            }),
        };
        (release, http, currentAsset)
    }

    /// Recreates Host storage and process state while preserving only the on-disk runtime tree.
    fn restart(&mut self) {
        let storage = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(
            self.root.clone(),
            self.root.join("workspaces"),
        ));
        self.context = HostManager::withFileSystemHost(Arc::new(
            operit_host_native_filesystem::PosixFileSystemHost::new(),
        ))
        .withRuntimeStorageHost(storage.clone())
        .withRuntimeStorageWriteHost(storage);
    }

    /// Completes one update attempt and reads its verified package through runtime storage.
    async fn download(&self, release: ReleaseInfo, current: String) -> Result<Vec<u8>, String> {
        let path = prepareUpdate(
            release,
            current,
            &self.context,
            HttpDownloadControl::new(),
            |_| {},
        )
        .await?;
        self.context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .map_err(|error| error.to_string())
    }

    /// Publishes a selected installer through the same VFS namespace as the Flutter picker.
    fn selected(&self, name: &str, bytes: &[u8]) -> String {
        let path = format!("runtime/temp/application_updates/selection/{name}");
        self.context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .writeBytes(&path, bytes)
            .unwrap();
        operit_vfs::VisualFileSystem::VisualFileSystem::runtimePath(&path).unwrap()
    }
}

/// Removed metadata and patches select a verified full package in this attempt.
#[tokio::test]
async fn missing_optional_assets_do_not_block_full_update() {
    for status in [404, 410] {
        for manifestMissing in [false, true] {
            let mut fixture = Fixture::new();
            let (release, http, current) = fixture.network(true, false);
            let missing = if manifestMissing {
                release.updateManifest.as_ref().unwrap().url.clone()
            } else {
                http.bodies
                    .keys()
                    .find(|url| url.ends_with(".patch.gz"))
                    .unwrap()
                    .clone()
            };
            http.statuses.lock().unwrap().insert(missing, status);
            let url = release.downloadUrl.clone();
            assert_eq!(
                fixture.download(release, current).await.unwrap(),
                fixture.target
            );
            assert_eq!(http.requests.lock().unwrap().last(), Some(&url));
            assert!(fixture
                .context
                .runtimeStorageHost
                .as_ref()
                .unwrap()
                .list("runtime/application_update_state")
                .unwrap()
                .into_iter()
                .filter(|entry| entry.path.ends_with(".json"))
                .collect::<Vec<_>>()
                .is_empty());
        }
    }
}

/// Full-only releases continue to update without requesting a manifest or patch.
#[tokio::test]
async fn release_without_incremental_metadata_uses_full_package() {
    let mut fixture = Fixture::new();
    let (mut release, http, current) = fixture.network(true, false);
    release.updateManifest = None;
    let url = release.downloadUrl.clone();
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(*http.requests.lock().unwrap(), vec![url]);
}

/// Server failures and corrupt metadata cannot be mistaken for intentionally absent optional files.
#[tokio::test]
async fn manifest_integrity_and_server_errors_remain_errors() {
    for corrupt in [false, true] {
        let mut fixture = Fixture::new();
        let (mut release, http, current) = fixture.network(true, false);
        if corrupt {
            release.updateManifest.as_mut().unwrap().digest = format!("sha256:{}", "0".repeat(64));
        } else {
            http.statuses
                .lock()
                .unwrap()
                .insert(release.updateManifest.as_ref().unwrap().url.clone(), 500);
        }
        assert!(fixture.download(release, current).await.is_err());
        assert_eq!(http.requests.lock().unwrap().len(), usize::from(corrupt));
    }
}

/// Missing complete packages report failure rather than producing an installable receipt.
#[tokio::test]
async fn missing_full_package_is_not_an_optional_resource() {
    let mut fixture = Fixture::new();
    let (mut release, http, current) = fixture.network(false, false);
    release.updateManifest = None;
    http.statuses
        .lock()
        .unwrap()
        .insert(release.downloadUrl.clone(), 404);
    assert!(fixture
        .download(release, current)
        .await
        .unwrap_err()
        .starts_with("HTTP 404"));
}

/// A manually distributed current installer survives restart and enables the first delta directly.
#[tokio::test]
async fn imported_current_original_enables_delta_without_a_previous_full_download() {
    let mut fixture = Fixture::new();
    let target =
        operit_util::GithubReleaseUtil::FullUpdateTarget::app("android", "arm64-v8a").unwrap();
    let current = fixture.manifest.patches[0].baseVersion.clone();
    let asset = target.assetNameForVersion(&current).unwrap();
    let selected = fixture.selected(&asset, &fixture.base.readBytes().unwrap());
    let imported = importLocalPackage(
        &selected,
        &current,
        &target,
        &fixture.context,
        &HttpDownloadControl::new(),
    )
    .unwrap();
    assert_eq!(imported.version, current);
    assert!(imported.packagePath.is_none());
    fixture.restart();
    let (release, http, current) = fixture.network(false, false);
    let mut bodies = http.bodies.clone();
    let patchUrl = format!(
        "{}/{}",
        release.downloadUrl.rsplit_once('/').unwrap().0,
        fixture.manifest.patches[0].assetName
    );
    bodies.insert(patchUrl.clone(), fixture.patch.readBytes().unwrap());
    let http = Arc::new(HttpFixture {
        bodies,
        requests: std::sync::Mutex::new(Vec::new()),
        failedUrl: std::sync::Mutex::new(None),
        statuses: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    });
    fixture.context = fixture.context.clone().withHttpHost(http.clone());
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1], patchUrl);
}

/// Local newer packages are retained once, ready for installation and the next delta.
#[test]
fn local_newer_package_prepares_an_offline_update() {
    let fixture = Fixture::new();
    let target =
        operit_util::GithubReleaseUtil::FullUpdateTarget::app("windows", "x86_64").unwrap();
    let name = target.assetNameForVersion("2.0.0-preview.2").unwrap();
    let source = fixture.selected(&name, &fixture.target);
    let imported = importLocalPackage(
        &source,
        "2.0.0-preview.1",
        &target,
        &fixture.context,
        &HttpDownloadControl::new(),
    )
    .unwrap();
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    let path = imported.packagePath.unwrap();
    assert_eq!(storage.readBytes(&path).unwrap(), fixture.target);
    assert!(path.starts_with("runtime/application_update_bases/"));
    assert!(storage
        .list("runtime/temp/application_updates")
        .unwrap()
        .iter()
        .all(|entry| uuid::Uuid::parse_str(entry.path.rsplit('/').next().unwrap()).is_err()));
    cleanupUpdateCache("2.0.0-preview.2", &name, &fixture.context).unwrap();
    assert!(storage.exists(&path).unwrap());
    assert_eq!(
        storage
            .readBytes(&format!(
                "runtime/application_update_bases/operit2-app-windows-x86_64.zip/{name}"
            ))
            .unwrap(),
        fixture.target
    );
}

/// MSI, macOS distributions and compound Linux archives retain their own exact package identities.
#[test]
fn local_import_supports_native_package_families() {
    use operit_host_api::{AppInstallTiming, ApplicationUpdatePackage, ApplicationUpdateTarget};
    use operit_util::GithubReleaseUtil::FullUpdateTarget;
    for (package, arch, header) in [
        (
            ApplicationUpdatePackage::WindowsMsi,
            "x86_64",
            &b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"[..],
        ),
        (
            ApplicationUpdatePackage::MacosPkg {
                buildType: "unsandbox".into(),
            },
            "aarch64",
            &b"xar!"[..],
        ),
        (
            ApplicationUpdatePackage::MacosPkg {
                buildType: "sandbox".into(),
            },
            "x86_64",
            &b"xar!"[..],
        ),
        (
            ApplicationUpdatePackage::LinuxTarGz,
            "aarch64",
            &b"\x1f\x8bxx"[..],
        ),
    ] {
        let fixture = Fixture::new();
        let target = FullUpdateTarget::fromApplicationTarget(ApplicationUpdateTarget {
            arch: arch.into(),
            package,
            installTiming: AppInstallTiming::Immediate,
        })
        .unwrap();
        let name = target.assetNameForVersion("2.0.0-preview.2").unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(b"original installer bytes");
        let source = fixture.selected(&name, &bytes);
        let imported = importLocalPackage(
            &source,
            "2.0.0-preview.1",
            &target,
            &fixture.context,
            &HttpDownloadControl::new(),
        )
        .unwrap();
        assert_eq!(
            fixture
                .context
                .runtimeStorageHost
                .as_ref()
                .unwrap()
                .readBytes(&imported.packagePath.unwrap())
                .unwrap(),
            bytes
        );
    }
}

/// Importing an original must not erase the target-specific decision made by a failed patch.
#[tokio::test]
async fn importing_a_baseline_preserves_a_previous_full_transfer_decision() {
    let mut fixture = Fixture::new();
    let (release, _, current) = fixture.network(true, true);
    assert!(fixture.download(release, current.clone()).await.is_err());
    let source = fixture.selected(&current, &fixture.base.readBytes().unwrap());
    let target =
        operit_util::GithubReleaseUtil::FullUpdateTarget::app("android", "arm64-v8a").unwrap();
    importLocalPackage(
        &source,
        &fixture.manifest.patches[0].baseVersion,
        &target,
        &fixture.context,
        &HttpDownloadControl::new(),
    )
    .unwrap();
    let (release, http, current) = fixture.network(true, false);
    let url = release.downloadUrl.clone();
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(*http.requests.lock().unwrap(), vec![url]);
}

/// Invalid, older, wrong-family and cancelled imports never publish a retained baseline.
#[test]
fn rejects_invalid_manual_packages_before_retaining_anything() {
    let fixture = Fixture::new();
    let target =
        operit_util::GithubReleaseUtil::FullUpdateTarget::app("windows", "x86_64").unwrap();
    for name in [
        "operit2-app-windows-x86_64-1.0.0.zip",
        "operit2-app-windows-aarch64-2.0.0.zip",
        "operit2-app-windows-x86_64-2.0.0.msi",
        "operit2-app-linux-x86_64-2.0.0.zip",
        "renamed.zip",
    ] {
        let source = fixture.selected(name, &fixture.target);
        assert!(importLocalPackage(
            &source,
            "2.0.0-preview.1",
            &target,
            &fixture.context,
            &HttpDownloadControl::new()
        )
        .is_err());
    }
    let source = fixture.selected(
        "operit2-app-windows-x86_64-2.0.0.zip",
        b"<html>broken download</html>",
    );
    assert!(importLocalPackage(
        &source,
        "2.0.0-preview.1",
        &target,
        &fixture.context,
        &HttpDownloadControl::new()
    )
    .is_err());
    let control = HttpDownloadControl::new();
    control.cancel();
    assert_eq!(
        importLocalPackage(
            &source,
            "2.0.0-preview.1",
            &target,
            &fixture.context,
            &control
        )
        .err()
        .unwrap(),
        "UPDATE_CANCELLED"
    );
    assert!(fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/application_update_bases")
        .unwrap()
        .is_empty());
}

/// A full transfer retains one package and removes its temporary download directory.
#[tokio::test]
async fn full_transfer_seeds_a_node_local_base() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(false, false);
    let path = prepareUpdate(
        release.clone(),
        current,
        &fixture.context,
        HttpDownloadControl::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
    assert_eq!(http.requests.lock().unwrap().len(), 2);
    assert!(path.starts_with("runtime/application_update_bases/"));
    cleanupUpdateCache(&release.version, &release.assetName, &fixture.context).unwrap();
    assert!(fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .exists(&path)
        .unwrap());
    let marker = format!("-{}.", release.version);
    let (family, extension) = release.assetName.split_once(&marker).unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&format!(
                "runtime/application_update_bases/{family}.{extension}/{}",
                release.assetName
            ))
            .unwrap(),
        fixture.target
    );
}

/// Startup removes interrupted work and obsolete state while keeping an uninstalled target intact.
#[test]
fn startup_cleans_only_obsolete_update_files() {
    let fixture = Fixture::new();
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    let root = "runtime/application_update_bases/operit2-app-android-arm64-v8a.apk";
    let current = "operit2-app-android-arm64-v8a-2.0.0-preview.2.apk";
    let old = format!("{root}/operit2-app-android-arm64-v8a-2.0.0-preview.1.apk");
    let future = format!("{root}/operit2-app-android-arm64-v8a-2.0.0-preview.3.apk");
    let abandoned = format!("runtime/temp/application_updates/{}", uuid::Uuid::new_v4());
    let policy = "runtime/application_update_state/operit2-app-android-arm64-v8a.apk.json";
    for path in [
        &old,
        &future,
        &format!("{root}/{current}"),
        &format!("{abandoned}/partial"),
        &format!("{root}/installer.log"),
        &format!("{root}/installer.finished"),
    ] {
        storage.writeBytes(path, b"fixture").unwrap();
    }
    storage
        .writeBytes(
            policy,
            br#"{"version":"2.0.0-preview.2","assetName":"unused"}"#,
        )
        .unwrap();
    cleanupUpdateCache("2.0.0-preview.2", current, &fixture.context).unwrap();
    assert!(storage.exists(&future).unwrap());
    assert!(storage.exists(&format!("{root}/{current}")).unwrap());
    let mut android = fixture.context.clone();
    android.systemOperationHost = Some(apk::application_host(String::new()));
    cleanupUpdateCache("2.0.0-preview.2", current, &android).unwrap();
    assert!(!storage.exists(&format!("{root}/{current}")).unwrap());
    for path in [
        &old,
        &abandoned,
        policy,
        &format!("{root}/installer.log"),
        &format!("{root}/installer.finished"),
    ] {
        assert!(!storage.exists(path).unwrap(), "{path}");
    }
    assert!(storage
        .exists("runtime/temp/application_updates/test/base.apk")
        .unwrap());
}

/// An exact retained base selects only the manifest and patch, then caches the original target bytes.
#[tokio::test]
async fn retained_base_selects_delta_without_full_transfer() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(true, false);
    let path = prepareUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .readBytes(&path)
            .unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].ends_with(".patch.gz"));
}

/// Stable targets reconstruct from both stable bases and preview bases accepted by the preview channel.
#[tokio::test]
async fn release_target_uses_stable_and_preview_base_deltas() {
    for baseVersion in ["1.9.0", "2.0.0-preview.1"] {
        let mut fixture = Fixture::new();
        let oldVersion = fixture.manifest.version.clone();
        fixture.manifest.version = "2.0.0".into();
        fixture.manifest.assetName = fixture.manifest.assetName.replace(&oldVersion, "2.0.0");
        fixture.manifest.patches[0].assetName = fixture.manifest.patches[0]
            .assetName
            .replace(&oldVersion, "2.0.0");
        fixture.manifest.patches[0].baseVersion = baseVersion.into();
        let (release, http, current) = fixture.network(true, false);
        let path = prepareUpdate(
            release,
            current,
            &fixture.context,
            HttpDownloadControl::new(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(
            fixture
                .context
                .runtimeStorageHost
                .as_ref()
                .unwrap()
                .readBytes(&path)
                .unwrap(),
            fixture.target
        );
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].ends_with(".patch.gz"));
    }
}

/// A selected corrupt patch records a future full transfer and cleans this attempt without another download.
#[tokio::test]
async fn selected_patch_failure_is_terminal_and_cleans_its_files() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(true, true);
    assert!(prepareUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
        |_| {}
    )
    .await
    .is_err());
    assert_eq!(http.requests.lock().unwrap().len(), 2);
    let entries = fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/temp/application_updates")
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].path.ends_with("/test"));
}

/// A failed target keeps using full transfers across restarts and repeated attempts for the same version.
#[tokio::test]
async fn failed_delta_forces_full_for_the_same_version_after_restart() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(true, true);
    let error = fixture.download(release, current).await.unwrap_err();
    assert!(error.ends_with("the next attempt for this version will download the full package"));
    assert_eq!(http.requests.lock().unwrap().len(), 2);
    let state = fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/application_update_state")
        .unwrap()
        .into_iter()
        .filter(|entry| entry.path.ends_with(".json"))
        .collect::<Vec<_>>();
    assert_eq!(state.len(), 1);
    assert_eq!(
        operit_host_api::RuntimeStorageLayout::runtimeStorageOwnership(&state[0].path).unwrap(),
        operit_host_api::RuntimeStorageLayout::RuntimeStorageOwnership::CoreNode
    );
    fixture.restart();
    for attempt in 0..2 {
        let (release, http, current) = fixture.network(true, false);
        let url = release.downloadUrl.clone();
        assert_eq!(
            fixture.download(release, current).await.unwrap(),
            fixture.target
        );
        assert_eq!(
            *http.requests.lock().unwrap(),
            if attempt == 0 {
                vec![url]
            } else {
                Vec::<String>::new()
            }
        );
    }
}

/// A baseline that cannot be hashed records the target before any patch bytes are transferred.
#[tokio::test]
async fn unreadable_baseline_makes_the_next_attempt_full() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(true, false);
    let versionSuffix = format!("-{}.", fixture.manifest.version);
    let family = fixture
        .manifest
        .assetName
        .split_once(&versionSuffix)
        .unwrap()
        .0;
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .writeBytes(
            &format!("runtime/application_update_bases/{family}.apk/{current}"),
            &[],
        )
        .unwrap();
    assert!(fixture.download(release, current).await.is_err());
    assert_eq!(http.requests.lock().unwrap().len(), 1);
    let (release, http, current) = fixture.network(true, false);
    let url = release.downloadUrl.clone();
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(*http.requests.lock().unwrap(), vec![url]);
}

/// Complete, authenticated patch bytes with an incorrect reconstructed target disable that target's deltas.
#[tokio::test]
async fn merge_verification_failure_makes_the_next_attempt_full() {
    use sha2::Digest;
    use std::io::{Read, Write};
    let mut fixture = Fixture::new();
    let original = fixture.patch.readBytes().unwrap();
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(original.as_slice())
        .read_to_end(&mut decoded)
        .unwrap();
    let lastLiteral = decoded.len() - 2;
    decoded[lastLiteral] ^= 1;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&decoded).unwrap();
    let corrupt = encoder.finish().unwrap();
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .writeBytes(
            "runtime/temp/application_updates/test/delta.patch.gz",
            &corrupt,
        )
        .unwrap();
    fixture.manifest.patches[0].sha256 = format!("{:x}", sha2::Sha256::digest(&corrupt));
    fixture.manifest.patches[0].size = corrupt.len() as u64;
    let (release, http, current) = fixture.network(true, false);
    assert!(fixture
        .download(release, current)
        .await
        .unwrap_err()
        .starts_with("Reconstructed package SHA-256 or length mismatch"));
    assert_eq!(http.requests.lock().unwrap().len(), 2);
    let (release, http, current) = fixture.network(true, false);
    let url = release.downloadUrl.clone();
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(*http.requests.lock().unwrap(), vec![url]);
}

/// Equal or larger patch transfers select the full package before consulting the retained baseline.
#[tokio::test]
async fn patches_at_least_as_large_as_the_full_package_are_not_used() {
    for extra in [0, 1] {
        let mut fixture = Fixture::new();
        fixture.manifest.patches[0].size = fixture.manifest.size + extra;
        let (release, http, current) = fixture.network(true, false);
        let url = release.downloadUrl.clone();
        assert_eq!(
            fixture.download(release, current).await.unwrap(),
            fixture.target
        );
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1], url);
    }
}

/// Alternative bases are separate direct routes, so unrelated deltas do not increase this transfer's cost.
#[tokio::test]
async fn alternative_base_patches_are_not_added_to_the_download_size() {
    let mut fixture = Fixture::new();
    fixture.manifest.patches.push(PackagePatch {
        baseVersion: "1.0.0".into(),
        baseSha256: "0".repeat(64),
        baseSize: 100,
        assetName: format!(
            "{}.from-{}.patch.gz",
            fixture.manifest.assetName,
            "0".repeat(64)
        ),
        sha256: "1".repeat(64),
        size: fixture.manifest.size - 1,
    });
    assert!(
        fixture
            .manifest
            .patches
            .iter()
            .map(|patch| patch.size)
            .sum::<u64>()
            > fixture.manifest.size
    );
    let (release, http, current) = fixture.network(true, false);
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].ends_with(".patch.gz"));
}

/// An interrupted HTTP transfer does not label the target's authentic patch as unusable.
#[tokio::test]
async fn network_failure_does_not_disable_delta_retry() {
    let mut fixture = Fixture::new();
    let (release, http, current) = fixture.network(true, false);
    *http.failedUrl.lock().unwrap() = Some(
        http.bodies
            .keys()
            .find(|url| url.ends_with(".patch.gz"))
            .unwrap()
            .clone(),
    );
    assert!(fixture.download(release, current).await.is_err());
    let (release, http, current) = fixture.network(true, false);
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].ends_with(".patch.gz"));
}

/// Cancelling local reconstruction leaves no durable full-download decision for the next attempt.
#[tokio::test]
async fn cancellation_does_not_disable_delta_retry() {
    use operit_util::GithubReleaseUtil::FullUpdateProgressEvent;
    let mut fixture = Fixture::new();
    let (release, _, current) = fixture.network(true, false);
    let control = HttpDownloadControl::new();
    let cancel = control.clone();
    let error = prepareUpdate(release, current, &fixture.context, control, move |event| {
        if let FullUpdateProgressEvent::StageChanged {
            stage: operit_util::GithubReleaseUtil::FullUpdateStage::ReconstructingPackage, ..
        } = event {
            cancel.cancel();
        }
    })
    .await
    .unwrap_err();
    assert_eq!(error, "UPDATE_CANCELLED");
    let (release, http, current) = fixture.network(true, false);
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    let requests = http.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].ends_with(".patch.gz"));
}

/// A failure in one target version cannot disable a newer target or a different architecture.
#[tokio::test]
async fn failed_delta_policy_is_scoped_to_one_target_and_package_family() {
    for differentFamily in [false, true] {
        let mut fixture = Fixture::new();
        let (release, _, current) = fixture.network(true, true);
        assert!(fixture.download(release, current).await.is_err());
        if differentFamily {
            fixture.manifest.assetName = format!(
                "operit2-app-android-x86_64-{}.apk",
                fixture.manifest.version
            );
        } else {
            let oldVersion = fixture.manifest.version.clone();
            fixture.manifest.version = "2.0.0-preview.3".into();
            fixture.manifest.assetName = fixture
                .manifest
                .assetName
                .replace(&oldVersion, &fixture.manifest.version);
        }
        fixture.manifest.patches[0].assetName = format!(
            "{}.from-{}.patch.gz",
            fixture.manifest.assetName, fixture.manifest.patches[0].baseSha256
        );
        let (release, http, current) = fixture.network(true, false);
        assert_eq!(
            fixture.download(release, current).await.unwrap(),
            fixture.target
        );
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].ends_with(".patch.gz"));
    }
}

/// A forced full transfer still authenticates every target byte and retains the target policy on failure.
#[tokio::test]
async fn forced_full_transfer_cannot_bypass_package_digest_checks() {
    let mut fixture = Fixture::new();
    let (release, _, current) = fixture.network(true, true);
    assert!(fixture.download(release, current).await.is_err());
    fixture.target.push(1);
    let (release, http, current) = fixture.network(true, false);
    assert_eq!(
        fixture.download(release, current).await.unwrap_err(),
        "UPDATE_INTEGRITY: downloaded update SHA-256 mismatch"
    );
    assert_eq!(http.requests.lock().unwrap().len(), 1);
    fixture.target.pop();
    let (release, http, current) = fixture.network(true, false);
    assert_eq!(
        fixture.download(release, current).await.unwrap(),
        fixture.target
    );
    assert_eq!(http.requests.lock().unwrap().len(), 1);
}
