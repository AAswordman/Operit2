use super::*;
use operit_util::GithubReleaseUtil::{FullUpdateProgressEvent, ReleaseManifestAsset};
use sha2::{Digest, Sha256};
use std::io::Write;

/// Hashes exact fixture bytes for both release metadata and patch envelopes.
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Generates real COPY/ADD streams independently of the production Rust decoder.
fn patch(base: &[u8], target: &[u8], padding: usize) -> Vec<u8> {
    assert!(target.starts_with(base));
    let mut gzip = flate2::GzBuilder::new()
        .comment(vec![b'x'; padding])
        .write(Vec::new(), flate2::Compression::default());
    gzip.write_all(b"O2PATCH1\0").unwrap();
    gzip.write_all(&(base.len() as u64).to_le_bytes()).unwrap();
    gzip.write_all(&(target.len() as u64).to_le_bytes())
        .unwrap();
    gzip.write_all(&Sha256::digest(base)).unwrap();
    gzip.write_all(&Sha256::digest(target)).unwrap();
    gzip.write_all(&[1]).unwrap();
    gzip.write_all(&0u64.to_le_bytes()).unwrap();
    gzip.write_all(&(base.len() as u64).to_le_bytes()).unwrap();
    gzip.write_all(&[2]).unwrap();
    gzip.write_all(&((target.len() - base.len()) as u64).to_le_bytes())
        .unwrap();
    gzip.write_all(&target[base.len()..]).unwrap();
    gzip.write_all(&[0]).unwrap();
    gzip.finish().unwrap()
}

/// Stages three authenticated historical hops, with optional direct and malformed routes.
fn network(
    fixture: &mut Fixture,
    full: bool,
    direct: Option<usize>,
    edit: impl FnOnce(&mut Vec<(UpdateManifest, Vec<u8>)>),
) -> (ReleaseInfo, Arc<HttpFixture>, String, u64) {
    let original = fixture.base.readBytes().unwrap();
    let mut base = original.clone();
    let mut hops = Vec::new();
    for version in 2..=4 {
        let mut target = base.clone();
        target.extend_from_slice(format!("signed installer version {version}").as_bytes());
        let delta = patch(&base, &target, 0);
        let asset = format!("operit2-app-android-arm64-v8a-2.0.0-preview.{version}.apk");
        let baseHash = digest(&base);
        let declaration = PackagePatch {
            baseVersion: format!("2.0.0-preview.{}", version - 1),
            baseSha256: baseHash.clone(),
            baseSize: base.len() as u64,
            assetName: format!("{asset}.from-{baseHash}.patch.gz"),
            sha256: digest(&delta),
            size: delta.len() as u64,
        };
        hops.push((
            UpdateManifest {
                schema: 1,
                version: format!("2.0.0-preview.{version}"),
                assetName: asset,
                sha256: digest(&target),
                size: target.len() as u64,
                patches: vec![declaration],
            },
            delta,
        ));
        base = target;
    }
    fixture.target = base;
    edit(&mut hops);
    let total = hops
        .iter()
        .map(|(manifest, _)| manifest.patches[0].size)
        .sum();
    let finalVersion = hops.last().unwrap().0.version.clone();
    let currentVersion = hops.first().unwrap().0.patches[0].baseVersion.clone();
    let current = hops
        .last()
        .unwrap()
        .0
        .assetName
        .replace(&format!("-{finalVersion}."), &format!("-{currentVersion}."));
    let mut bodies = std::collections::BTreeMap::new();
    if let Some(padding) = direct {
        let manifest = &mut hops.last_mut().unwrap().0;
        let delta = patch(&original, &fixture.target, padding);
        let baseHash = digest(&original);
        let name = format!("{}.from-{baseHash}.patch.gz", manifest.assetName);
        bodies.insert(
            format!(
                "https://github.com/AAswordman/Operit2-nightly/releases/download/v{}/{name}",
                manifest.version
            ),
            delta.clone(),
        );
        manifest.patches.push(PackagePatch {
            baseVersion: "2.0.0-preview.1".into(),
            baseSha256: baseHash,
            baseSize: original.len() as u64,
            assetName: name,
            sha256: digest(&delta),
            size: delta.len() as u64,
        });
    }
    let mut selected = None;
    for (manifest, bytes) in hops {
        let root = format!(
            "https://github.com/AAswordman/Operit2-nightly/releases/download/v{}",
            manifest.version
        );
        let metadata = serde_json::to_vec(&manifest).unwrap();
        let manifestName = format!("{}.update.json", manifest.assetName);
        let manifestUrl = format!("{root}/{manifestName}");
        let hash = format!("sha256:{}", digest(&metadata));
        let assets = std::iter::once(serde_json::json!({
            "name": manifestName, "browser_download_url": manifestUrl, "digest": hash,
        })).chain(manifest.patches.iter().map(|patch| serde_json::json!({
            "name": patch.assetName, "browser_download_url": format!("{root}/{}", patch.assetName),
            "digest": format!("sha256:{}", patch.sha256),
        }))).collect::<Vec<_>>();
        bodies.insert(format!("https://api.github.com/repos/AAswordman/Operit2-nightly/releases/tags/v{}", manifest.version),
            serde_json::to_vec(&serde_json::json!({
                "tag_name": format!("v{}", manifest.version), "body": "patches only", "html_url": root,
                "draft": false, "prerelease": operit_util::GithubReleaseUtil::GithubReleaseUtil::fullUpdateChannelForVersion(&manifest.version).unwrap()
                    == operit_util::GithubReleaseUtil::FullUpdateChannel::Preview, "assets": assets,
            })).unwrap());
        bodies.insert(manifestUrl.clone(), metadata);
        bodies.insert(format!("{root}/{}", manifest.patches[0].assetName), bytes);
        if manifest.version == finalVersion {
            let fullUrl = format!(
                "https://github.com/AAswordman/Operit2/releases/download/v{}/{}",
                manifest.version, manifest.assetName
            );
            bodies.insert(fullUrl.clone(), fixture.target.clone());
            selected = Some(ReleaseInfo {
                version: manifest.version.clone(),
                assetName: manifest.assetName.clone(),
                downloadUrl: fullUrl,
                releaseNotes: String::new(),
                releasePageUrl: root,
                packageSha256: full.then(|| format!("sha256:{}", manifest.sha256)),
                fullPackageAvailable: full,
                patchAssetNames: manifest
                    .patches
                    .iter()
                    .map(|patch| patch.assetName.clone())
                    .collect(),
                updateManifest: Some(ReleaseManifestAsset {
                    name: manifestName,
                    url: manifestUrl,
                    digest: hash,
                }),
            });
            fixture.manifest = manifest;
        }
    }
    fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .writeBytes(
            &format!(
                "runtime/application_update_bases/operit2-app-android-arm64-v8a.apk/{current}"
            ),
            &original,
        )
        .unwrap();
    let http = Arc::new(HttpFixture {
        bodies,
        requests: Default::default(),
        failedUrl: Default::default(),
        statuses: Default::default(),
    });
    fixture.context = fixture
        .context
        .clone()
        .withHttpHost(http.clone())
        .withHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
        ));
    (selected.unwrap(), http, current, total)
}

/// Three adjacent patches reconstruct the exact target with aggregate progress and disposable intermediates.
#[tokio::test]
async fn adjacent_chain_reconstructs_and_cleans_every_intermediate() {
    for full in [false, true] {
        let mut fixture = Fixture::new();
        let (release, http, current, total) = network(&mut fixture, full, None, |_| {});
        let control = HttpDownloadControl::new();
        let plan = planUpdate(release, current, &fixture.context, control.clone())
            .await
            .unwrap();
        assert_eq!(plan.mode, UpdateMode::Incremental);
        assert_eq!(plan.incrementalBytes, Some(total));
        assert!(http
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|url| !url.ends_with(".patch.gz")));
        let progress = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = progress.clone();
        let path = executeUpdate(plan, &fixture.context, control, move |event| {
            if let FullUpdateProgressEvent::DownloadProgress {
                readBytes,
                totalBytes,
                ..
            } = event
            {
                sink.lock().unwrap().push((readBytes, totalBytes));
            }
        })
        .await
        .unwrap();
        let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
        assert_eq!(storage.readBytes(&path).unwrap(), fixture.target);
        let progress = progress.lock().unwrap();
        assert!(!progress.is_empty());
        assert!(progress.iter().all(|(_, size)| *size == total));
        assert!(progress.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert_eq!(progress.last().unwrap().0, total);
        assert_eq!(
            http.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|url| url.ends_with(".patch.gz"))
                .count(),
            3
        );
        let jobs = storage.list("runtime/temp/application_updates").unwrap();
        assert!(jobs
            .iter()
            .all(|job| uuid::Uuid::parse_str(job.path.rsplit('/').next().unwrap()).is_err()));
        let cached = storage
            .list("runtime/application_update_bases/operit2-app-android-arm64-v8a.apk")
            .unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].path, path);
    }
}

/// The smallest download wins even when an adjacent chain and a direct edge both exist.
#[tokio::test]
async fn cheapest_download_wins_between_direct_and_chained_patches() {
    for padding in [0, 1024] {
        let mut fixture = Fixture::new();
        let (release, http, current, chainBytes) =
            network(&mut fixture, true, Some(padding), |_| {});
        let plan = planUpdate(
            release,
            current,
            &fixture.context,
            HttpDownloadControl::new(),
        )
        .await
        .unwrap();
        if padding == 0 {
            assert!(plan.incrementalBytes.unwrap() < chainBytes);
        } else {
            assert_eq!(plan.incrementalBytes, Some(chainBytes));
        }
        executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
            .await
            .unwrap();
        assert_eq!(
            http.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|url| url.ends_with(".patch.gz"))
                .count(),
            if padding == 0 { 1 } else { 3 }
        );
    }
}

/// Total route bytes, rather than each individual edge, determine the full-package choice.
#[tokio::test]
async fn chain_at_or_above_full_size_uses_full_only_when_published() {
    for full in [false, true] {
        for extra in [0, 1] {
            let mut fixture = Fixture::new();
            let (release, http, current, _) = network(&mut fixture, full, None, |hops| {
                let targetSize = hops.last().unwrap().0.size;
                for (index, (manifest, _)) in hops.iter_mut().enumerate() {
                    manifest.patches[0].size = targetSize / 3
                        + if index == 2 {
                            targetSize % 3 + extra
                        } else {
                            0
                        };
                }
            });
            let plan = planUpdate(
                release,
                current,
                &fixture.context,
                HttpDownloadControl::new(),
            )
            .await
            .unwrap();
            assert_eq!(
                plan.mode,
                if full {
                    UpdateMode::Full
                } else {
                    UpdateMode::Incremental
                }
            );
            if full {
                executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
                    .await
                    .unwrap();
                assert!(http
                    .requests
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|url| !url.ends_with(".patch.gz")));
            }
        }
    }
}

/// A missing historical release or mismatched intermediate digest cannot create an executable route.
#[tokio::test]
async fn missing_or_mismatched_intermediate_stops_patch_only_update() {
    for missing in [false, true] {
        let mut fixture = Fixture::new();
        let (release, http, current, _) = network(&mut fixture, false, None, |hops| {
            if !missing {
                hops[1].0.patches[0].baseSha256 = "0".repeat(64);
                let asset = hops[1].0.assetName.clone();
                let patch = &mut hops[1].0.patches[0];
                patch.assetName = format!("{asset}.from-{}.patch.gz", patch.baseSha256);
            }
        });
        if missing {
            http.statuses.lock().unwrap().insert(
            "https://api.github.com/repos/AAswordman/Operit2-nightly/releases/tags/v2.0.0-preview.2".into(), 404);
        }
        let plan = planUpdate(
            release,
            current,
            &fixture.context,
            HttpDownloadControl::new(),
        )
        .await
        .unwrap();
        assert_eq!(plan.mode, UpdateMode::Unavailable);
        assert!(http
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|url| !url.ends_with(".patch.gz")));
    }
}

/// Import validates the original once before any patch in the chain can be downloaded.
#[tokio::test]
async fn chain_prompts_for_the_installed_original_and_rejects_wrong_bytes() {
    let mut fixture = Fixture::new();
    let (release, http, current, total) = network(&mut fixture, false, None, |_| {});
    removeRetainedBase(&fixture, &release, &current);
    let control = HttpDownloadControl::new();
    let mut plan = planUpdate(release, current, &fixture.context, control.clone())
        .await
        .unwrap();
    assert_eq!(plan.mode, UpdateMode::NeedsBase);
    assert_eq!(plan.baseVersion.as_deref(), Some("2.0.0-preview.1"));
    assert_eq!(plan.incrementalBytes, Some(total));
    let wrong = fixture.selected("wrong.apk", b"PK\x03\x04wrong");
    assert!(plan
        .supplyBase(&wrong, &fixture.context, &control)
        .unwrap_err()
        .starts_with("UPDATE_BASE_MISMATCH"));
    assert!(http
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|url| !url.ends_with(".patch.gz")));
    let original = fixture.selected("original.apk", &fixture.base.readBytes().unwrap());
    plan.supplyBase(&original, &fixture.context, &control)
        .unwrap();
    executeUpdate(plan, &fixture.context, control, |_| {})
        .await
        .unwrap();
}

/// A verified patch with invalid instructions records full-only policy for the final version, not an intermediate.
#[tokio::test]
async fn middle_merge_failure_records_final_target_policy_and_cleans_jobs() {
    let mut fixture = Fixture::new();
    let (release, http, current, _) = network(&mut fixture, true, None, |hops| {
        let (manifest, bytes) = &mut hops[1];
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(b"invalid patch instructions").unwrap();
        *bytes = gzip.finish().unwrap();
        manifest.patches[0].sha256 = digest(bytes);
        manifest.patches[0].size = bytes.len() as u64;
    });
    let plan = planUpdate(
        release.clone(),
        current.clone(),
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    let error = executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
        .await
        .unwrap_err();
    assert!(error.contains("next attempt for this version"), "{error}");
    assert_eq!(
        http.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|url| url.ends_with(".patch.gz"))
            .count(),
        2
    );
    http.requests.lock().unwrap().clear();
    let plan = planUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.mode, UpdateMode::Full);
    executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
        .await
        .unwrap();
    assert!(http
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|url| !url.ends_with(".patch.gz")));
    assert!(fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/temp/application_updates")
        .unwrap()
        .iter()
        .all(|job| uuid::Uuid::parse_str(job.path.rsplit('/').next().unwrap()).is_err()));
}

/// Cancellation between hops cleans temporary files, retains the original, and never forces full transfer.
#[tokio::test]
async fn cancelling_the_second_merge_preserves_incremental_retry() {
    let mut fixture = Fixture::new();
    let (release, _, current, _) = network(&mut fixture, false, None, |_| {});
    let control = HttpDownloadControl::new();
    let plan = planUpdate(
        release.clone(),
        current.clone(),
        &fixture.context,
        control.clone(),
    )
    .await
    .unwrap();
    let merges = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let cancel = control.clone();
    let error = executeUpdate(plan, &fixture.context, control, move |event| {
        if let FullUpdateProgressEvent::StageChanged {
            stage: operit_util::GithubReleaseUtil::FullUpdateStage::ReconstructingPackage,
            ..
        } = event
        {
            if merges.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                cancel.cancel();
            }
        }
    })
    .await
    .unwrap_err();
    assert_eq!(error, "UPDATE_CANCELLED");
    assert!(fixture
        .context
        .runtimeStorageHost
        .as_ref()
        .unwrap()
        .list("runtime/temp/application_updates")
        .unwrap()
        .iter()
        .all(|job| uuid::Uuid::parse_str(job.path.rsplit('/').next().unwrap()).is_err()));
    let plan = planUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.mode, UpdateMode::Incremental);
    executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {})
        .await
        .unwrap();
}

/// A middle patch removed after planning uses only a confirmed full asset and releases partial results.
#[tokio::test]
async fn removed_middle_patch_respects_full_availability() {
    for full in [false, true] {
        let mut fixture = Fixture::new();
        let (release, http, current, _) = network(&mut fixture, full, None, |_| {});
        let fullUrl = release.downloadUrl.clone();
        let missing = http
            .bodies
            .keys()
            .find(|url| {
                url.ends_with(".patch.gz") && url.split('/').nth(7) == Some("v2.0.0-preview.3")
            })
            .unwrap()
            .clone();
        let plan = planUpdate(
            release,
            current,
            &fixture.context,
            HttpDownloadControl::new(),
        )
        .await
        .unwrap();
        http.statuses.lock().unwrap().insert(missing, 404);
        let result =
            executeUpdate(plan, &fixture.context, HttpDownloadControl::new(), |_| {}).await;
        if full {
            assert_eq!(
                fixture
                    .context
                    .runtimeStorageHost
                    .as_ref()
                    .unwrap()
                    .readBytes(&result.unwrap())
                    .unwrap(),
                fixture.target
            );
            assert_eq!(http.requests.lock().unwrap().last(), Some(&fullUrl));
        } else {
            assert!(result.unwrap_err().starts_with("UPDATE_UNAVAILABLE"));
            assert!(!http
                .requests
                .lock()
                .unwrap()
                .iter()
                .any(|url| url == &fullUrl));
        }
        assert!(fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .list("runtime/temp/application_updates")
            .unwrap()
            .iter()
            .all(|job| uuid::Uuid::parse_str(job.path.rsplit('/').next().unwrap()).is_err()));
    }
}

/// Stable clients reject a preview intermediate even when its bytes would otherwise connect exactly.
#[tokio::test]
async fn stable_route_never_traverses_a_preview_package() {
    let mut fixture = Fixture::new();
    let (release, http, current, _) = network(&mut fixture, true, None, |hops| {
        let versions = ["2.0.1", "2.0.2-preview.1", "2.0.2", "2.0.3"];
        for (index, (manifest, _)) in hops.iter_mut().enumerate() {
            manifest.assetName = manifest
                .assetName
                .replace(&manifest.version, versions[index + 1]);
            manifest.version = versions[index + 1].into();
            manifest.patches[0].baseVersion = versions[index].into();
            manifest.patches[0].assetName = format!(
                "{}.from-{}.patch.gz",
                manifest.assetName, manifest.patches[0].baseSha256
            );
        }
    });
    let plan = planUpdate(
        release,
        current,
        &fixture.context,
        HttpDownloadControl::new(),
    )
    .await
    .unwrap();
    assert_eq!(plan.mode, UpdateMode::Full);
    assert!(!plan.incrementalAvailable);
    assert!(http
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|url| !url.ends_with("/v2.0.2-preview.1")));
}
