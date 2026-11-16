use super::*;
use operit_host_api::*;
use std::path::Path;

struct ApkHost;

/// Defines unused system operations without introducing test implementations of unrelated behavior.
macro_rules! unused {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $out:ty;)*) => {$ (
        #[doc = "Rejects unrelated system operations in the APK reconstruction fixture."]
        fn $name(&self, $(_: $ty),*) -> HostResult<$out> { panic!(stringify!($name)) }
    )*};
}

impl SystemOperationHost for ApkHost {
    /// Exercises the production Android transform over the same VFS files used by the updater.
    fn transformApplicationUpdate(
        &self,
        source: &dyn FileSource,
        target: &dyn FileTarget,
        transform: ApplicationUpdateTransform,
        control: &HttpDownloadControl,
    ) -> HostResult<()> {
        apk_host::transform(
            Path::new(&source.nativePath()?),
            Path::new(&target.nativePath()?),
            transform,
            control,
        )
        .map_err(|e| HostError::new(e.to_string()))
    }

    unused! {
        getSystemLanguageCode() -> String;
        sendNotification(request: &SystemNotificationRequest) -> ();
        modifySystemSetting(namespace: &str, setting: &str, value: &str) -> SystemSettingData;
        getSystemSetting(namespace: &str, setting: &str) -> SystemSettingData;
        installApp(source: &dyn FileSource, timing: AppInstallTiming) -> AppOperationData;
        uninstallApp(packageName: &str) -> AppOperationData;
        listInstalledApps(includeSystemApps: bool) -> AppListData;
        startApp(packageName: &str) -> AppOperationData;
        stopApp(packageName: &str) -> AppOperationData;
        getNotifications(limit: i32, includeOngoing: bool) -> NotificationData;
        getAppUsageTime(packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> AppUsageTimeResultData;
        getDeviceLocation(timeout: i32, highAccuracy: bool, includeAddress: bool) -> LocationData;
        getDeviceInfo() -> DeviceInfoData;
        captureScreenshot() -> String;
        recognizeText(imagePath: &dyn FileSource, language: OCRLanguage, quality: OCRQuality) -> String;
    }
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
    fixture.context.systemOperationHost = Some(Arc::new(ApkHost));
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
    fixture
}

/// Recompression must preserve signature shell bytes and the target hash across Python and Rust.
#[test]
fn apk_python_delta_reconstructs_exact_original_through_android_host() {
    let fixture = fixture();
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
    assert!(fixture.manifest.patches[0].size < fixture.manifest.size / 3);
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    assert!(storage
        .list("runtime/temp/application_updates")
        .unwrap()
        .iter()
        .all(|entry| entry.path.ends_with("/test")));
}

/// A signed recipe cannot request overlapping ranges or compression outside the supported codec.
#[test]
fn apk_host_rejects_invalid_recipe_without_retaining_output() {
    let fixture = fixture();
    let target = fixture.root.join("invalid-expanded");
    for recipe in [
        serde_json::json!({"size": 100, "entries": [{"offset": 99,"compressed": 2,"size": 1,"level":6}]}),
        serde_json::json!({"size": 100, "entries": [{"offset": 0,"compressed": 10,"size": 1,"level":9}]}),
        serde_json::json!({"size": 0, "entries": []}),
    ] {
        assert!(apk_host::transform(
            Path::new(&FileSource::nativePath(&fixture.base).unwrap()),
            &target,
            ApplicationUpdateTransform::ExpandApkEntries {
                recipe: serde_json::to_vec(&recipe).unwrap()
            },
            &HttpDownloadControl::new()
        )
        .is_err());
        assert!(!target.exists());
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
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(
        fixture.output(),
        std::fs::read(std::env::var("OPERIT_APK_TEST_TARGET").unwrap()).unwrap()
    );
}
