use super::*;
use operit_host_api::*;
use super::flutter_system_operation::{FlutterSystemBindings, FlutterSystemOperationBridge};

/// Assembles the production Android system host behind the same Flutter decorator used by the app.
pub(super) fn application_host(path: String) -> Arc<dyn SystemOperationHost> {
    let host = android_system_operation::AndroidSystemOperationHost::new(
        Arc::new(|_, _| panic!("unexpected settings read")),
        Arc::new(|_, _, _| panic!("unexpected settings write")),
        Arc::new(move |operation, params| match operation {
            "application_update_base" => {
                assert!(!path.is_empty(), "cache cleanup must not request an installed APK from the owner");
                Ok(serde_json::json!({"path": path}))
            }
            "install_app" => {
                assert!(std::path::Path::new(params["path"].as_str().unwrap()).is_file());
                Ok(serde_json::json!({
                    "operationType": "install_app", "packageName": "app.operit",
                    "success": true, "details": "Installer opened by fixture owner",
                    "installState": "installer_opened",
                }))
            }
            _ => panic!("Unexpected Android owner operation: {operation}"),
        }),
    );
    Arc::new(FlutterSystemOperationBridge::new(Arc::new(host), FlutterSystemBindings {
        notificationSender: Arc::new(|_| panic!("unexpected notification")),
        notifications: Arc::new(|_, _| panic!("unexpected notifications read")),
        location: Arc::new(|_, _, _| panic!("unexpected location")),
        deviceInfo: Arc::new(|| panic!("unexpected device info")),
        screenshot: Arc::new(|| panic!("unexpected screenshot")),
        recognition: Arc::new(|_, _, _| panic!("unexpected OCR")),
    }))
}

/// Loads an independently Python-generated APK delta through real Android compression and shared VFS.
fn fixture() -> Fixture {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/package_delta_apk.json")).unwrap();
    let mut fixture = Fixture::new();
    let bytes = |name: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(data[name].as_str().unwrap())
            .unwrap()
    };
    fixture.manifest = serde_json::from_value(data["manifest"].clone()).unwrap();
    fixture.target = bytes("target");
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    storage
        .writeBytes(
            "runtime/temp/application_updates/test/base.apk",
            &bytes("base"),
        )
        .unwrap();
    storage
        .writeBytes(
            "runtime/temp/application_updates/test/delta.patch.gz",
            &bytes("patch"),
        )
        .unwrap();
    fixture.context.systemOperationHost = Some(application_host(FileSource::nativePath(&fixture.base).unwrap()));
    fixture
}

/// The production Android codec forwards all local phases through the Flutter decorator.
#[tokio::test]
async fn apk_update_reports_expansion_and_recompression_progress() {
    use operit_util::GithubReleaseUtil::UpdatePreparationPhase::*;
    super::preparation_progress::phases(fixture(), &[
        VerifyingBase, VerifyingPatch, ExpandingBase, VerifyingExpandedBase,
        ApplyingPatch, RestoringPackage, VerifyingPackage, SavingPackage,
    ]).await;
}

/// Recompression must preserve signature shell bytes and the target hash across Python and Rust.
#[test]
fn apk_python_delta_reconstructs_exact_original_through_android_host() {
    let fixture = fixture();
    let host = fixture.context.systemOperationHost.as_ref().unwrap();
    let ApplicationUpdateBase::InstalledPackage(base) = host.applicationUpdateBase().unwrap() else {
        panic!("Android must expose its installed APK");
    };
    assert!(base.nativePath().is_err(), "Installed APK must remain a read-only capability");
    applyPatch(base.as_ref(), &fixture.patch, &fixture.manifest, &fixture.manifest.patches[0],
        &fixture.context, "runtime/temp/application_updates/test/output.apk", &HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
    assert!(fixture.manifest.patches[0].size < fixture.manifest.size / 3);
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    assert!(storage
        .list("runtime/temp/application_updates")
        .unwrap()
        .iter()
        .all(|entry| entry.path.ends_with("/test")));
}

/// Plans and executes a patch-only update from the installed APK through the real Flutter Host chain.
#[tokio::test]
async fn apk_patch_only_update_reaches_installer_through_flutter_bridge() {
    let mut fixture = fixture();
    let (mut release, http, current) = fixture.network(true, false);
    release.fullPackageAvailable = false;
    release.packageSha256 = None;
    removeRetainedBase(&fixture, &release, &current);
    let target = fixture.context.systemOperationHost.as_ref().unwrap().applicationUpdateTarget().unwrap().unwrap();
    assert_eq!(target.package, ApplicationUpdatePackage::AndroidApk);
    let control = HttpDownloadControl::new();
    let plan = planUpdate(release.clone(), current, &fixture.context, control.clone()).await.unwrap();
    assert_eq!(plan.mode, UpdateMode::Incremental);
    assert!(!plan.fullAvailable);
    let path = executeUpdate(plan, &fixture.context, control, |_| {}).await.unwrap();
    assert_eq!(fixture.context.runtimeStorageHost.as_ref().unwrap().readBytes(&path).unwrap(), fixture.target);
    assert!(!http.requests.lock().unwrap().contains(&release.downloadUrl));
    let source = fixture.context.vfs().unwrap().file(&operit_vfs::VisualFileSystem::VisualFileSystem::runtimePath(&path).unwrap()).unwrap();
    let result = fixture.context.systemOperationHost.as_ref().unwrap().installApp(&source, target.installTiming).unwrap();
    assert!(result.success);
    assert_eq!(result.installState, Some(AppInstallState::InstallerOpened));
}

/// Cancellation and an invalid recipe must leave no output even when routed through the Flutter decorator.
#[test]
fn apk_flutter_bridge_preserves_transform_failures_and_cancellation() {
    let fixture = fixture();
    let host = fixture.context.systemOperationHost.as_ref().unwrap();
    let target = fixture.context.vfs().unwrap().file("/operitapp/data/temp/application_updates/test/rejected").unwrap();
    let target_path = FileTarget::nativePath(&target).unwrap();
    let control = HttpDownloadControl::new();
    control.cancel();
    let error = host.transformApplicationUpdate(&fixture.base, &target,
        ApplicationUpdateTransform::ExpandApkEntries { recipe: b"{}".to_vec() }, &control, &|_, _| {}).unwrap_err();
    assert!(error.to_string().contains("UPDATE_CANCELLED"));
    assert!(!std::path::Path::new(&target_path).exists());
    assert!(host.transformApplicationUpdate(&fixture.base, &target,
        ApplicationUpdateTransform::ExpandApkEntries { recipe: b"{}".to_vec() }, &HttpDownloadControl::new(), &|_, _| {}).is_err());
    assert!(!std::path::Path::new(&target_path).exists());
}

/// A signed recipe cannot request overlapping ranges or compression outside the supported codec.
#[test]
fn apk_host_rejects_invalid_recipe_without_retaining_output() {
    let fixture = fixture();
    let target = fixture.context.vfs().unwrap().file("/operitapp/data/temp/application_updates/test/invalid-expanded").unwrap();
    let target_path = FileTarget::nativePath(&target).unwrap();
    for recipe in [
        serde_json::json!({"size": 100, "entries": [{"offset": 99,"compressed": 2,"size": 1,"level":6}]}),
        serde_json::json!({"size": 100, "entries": [{"offset": 0,"compressed": 10,"size": 1,"level":9}]}),
        serde_json::json!({"size": 0, "entries": []}),
    ] {
        assert!(update_apk::transform(
            &fixture.base,
            &target,
            ApplicationUpdateTransform::ExpandApkEntries {
                recipe: serde_json::to_vec(&recipe).unwrap()
            },
            &HttpDownloadControl::new(),
            &|_, _| {},
        )
        .is_err());
        assert!(!std::path::Path::new(&target_path).exists());
    }
}

/// APK-aware data requires schema 4 and cannot be interpreted as the older residual format.
#[test]
fn apk_format_is_bound_to_its_manifest_and_cancellation() {
    let mut fixture = fixture();
    fixture.manifest.schema = 3;
    assert!(fixture
        .apply(&HttpDownloadControl::new())
        .unwrap_err()
        .contains("header"));
    fixture.manifest.schema = 4;
    let control = HttpDownloadControl::new();
    control.cancel();
    assert_eq!(fixture.apply(&control).unwrap_err(), "UPDATE_CANCELLED");
}

/// Validates release-sized APKs with the real Android Host without accessing a device.
#[test]
#[ignore = "requires explicitly supplied local release APKs and a generated manifest"]
fn apk_real_release_round_trip() {
    let mut fixture = fixture();
    fixture.manifest = serde_json::from_slice(
        &std::fs::read(std::env::var("OPERIT_APK_TEST_MANIFEST").unwrap()).unwrap(),
    )
    .unwrap();
    std::fs::copy(
        std::env::var("OPERIT_APK_TEST_BASE").unwrap(),
        FileSource::nativePath(&fixture.base).unwrap(),
    )
    .unwrap();
    std::fs::copy(
        std::env::var("OPERIT_APK_TEST_PATCH").unwrap(),
        FileSource::nativePath(&fixture.patch).unwrap(),
    )
    .unwrap();
    let host = fixture.context.systemOperationHost.as_ref().unwrap();
    let ApplicationUpdateBase::InstalledPackage(base) = host.applicationUpdateBase().unwrap() else {
        panic!("Android must expose its installed APK");
    };
    assert!(base.nativePath().is_err());
    applyPatch(base.as_ref(), &fixture.patch, &fixture.manifest, &fixture.manifest.patches[0],
        &fixture.context, "runtime/temp/application_updates/test/output.apk", &HttpDownloadControl::new()).unwrap();
    assert_eq!(
        fixture.output(),
        std::fs::read(std::env::var("OPERIT_APK_TEST_TARGET").unwrap()).unwrap()
    );
}
