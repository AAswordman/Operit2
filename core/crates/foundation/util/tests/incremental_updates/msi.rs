use super::*;
use operit_host_api::*;
use std::io::Read;
use std::path::Path;

struct MsiHost;

/// Parallel Windows CAB work reports bounded counters through the same Core update stream.
#[tokio::test]
async fn msi_update_reports_expansion_and_recompression_progress() {
    use operit_util::GithubReleaseUtil::UpdatePreparationPhase::*;
    super::preparation_progress::phases(
        fixture(),
        &[
            VerifyingBase,
            VerifyingPatch,
            ExpandingBase,
            VerifyingExpandedBase,
            ApplyingPatch,
            RestoringPackage,
            VerifyingPackage,
            SavingPackage,
        ],
    )
    .await;
}

/// Keeps unrelated platform capabilities outside the isolated installer-codec test.
macro_rules! unused_operations {
    ($(fn $name:ident(&self $(, $arg:ident: $ty:ty)*) -> $result:ty;)*) => {
        $(
            #[doc = "Rejects unrelated system operations in the isolated codec test host."]
            fn $name(&self $(, $arg: $ty)*) -> $result {
                $(let _ = $arg;)*
                panic!(concat!("unexpected system operation: ", stringify!($name)))
            }
        )*
    };
}

impl SystemOperationHost for MsiHost {
    /// Calls the production Windows codec with the same native capability boundary as its owner.
    fn transformApplicationUpdate(
        &self,
        source: &dyn FileSource,
        target: &dyn FileTarget,
        transform: ApplicationUpdateTransform,
        control: &HttpDownloadControl,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> HostResult<()> {
        update_msi::transform(
            Path::new(&source.nativePath()?),
            Path::new(&target.nativePath()?),
            transform,
            control,
            progress,
        )
        .map_err(|error| HostError::new(error.to_string()))
    }

    unused_operations! {
        fn getSystemLanguageCode(&self) -> HostResult<String>;
        fn sendNotification(&self, request: &SystemNotificationRequest) -> HostResult<()>;
        fn modifySystemSetting(&self, namespace: &str, setting: &str, value: &str) -> HostResult<SystemSettingData>;
        fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData>;
        fn installApp(&self, source: &dyn FileSource, timing: AppInstallTiming) -> HostResult<AppOperationData>;
        fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData>;
        fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData>;
        fn startApp(&self, packageName: &str) -> HostResult<AppOperationData>;
        fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData>;
        fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData>;
        fn getAppUsageTime(&self, packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> HostResult<AppUsageTimeResultData>;
        fn getDeviceLocation(&self, timeout: i32, highAccuracy: bool, includeAddress: bool) -> HostResult<LocationData>;
        fn getDeviceInfo(&self) -> HostResult<DeviceInfoData>;
        fn captureScreenshot(&self) -> HostResult<String>;
        fn recognizeText(&self, imagePath: &dyn FileSource, language: OCRLanguage, quality: OCRQuality) -> HostResult<String>;
    }
}

/// Loads Python-generated CAB deltas without linking unrelated native inference dependencies.
fn fixture() -> Fixture {
    let mut fixture = Fixture::new();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tools/tests/fixtures/msi_cabinet_delta.json"
    ))
    .unwrap();
    let decode = |key: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(data[key].as_str().unwrap())
            .unwrap()
    };
    fixture.manifest = serde_json::from_value(data["manifest"].clone()).unwrap();
    fixture.target = decode("target");
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    storage
        .writeBytes(
            "runtime/temp/application_updates/test/base.apk",
            &decode("base"),
        )
        .unwrap();
    storage
        .writeBytes(
            "runtime/temp/application_updates/test/delta.patch.gz",
            &decode("patch"),
        )
        .unwrap();
    fixture.context.systemOperationHost = Some(Arc::new(MsiHost));
    fixture
}

/// The shared batching decoder must preserve exact MSI bytes through the real Windows CAB codec.
#[test]
fn msi_python_delta_reconstructs_exact_original_through_windows_codec() {
    let fixture = fixture();
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(fixture.output(), fixture.target);
    assert_eq!(
        fixture
            .context
            .runtimeStorageHost
            .as_ref()
            .unwrap()
            .list("runtime/temp/application_updates")
            .unwrap()
            .len(),
        1
    );
}

/// Independent folders may finish out of order without changing shell, payload, or signed bytes.
#[test]
fn parallel_cab_folders_preserve_layout_and_clean_up_worker_failures() {
    let fixture = fixture();
    let base = fixture.base.readBytes().unwrap();
    let mut decoder =
        flate2::read::MultiGzDecoder::new(std::io::Cursor::new(fixture.patch.readBytes().unwrap()));
    decoder.read_exact(&mut [0; 89]).unwrap();
    let mut length = [0; 4];
    decoder.read_exact(&mut length).unwrap();
    let mut metadata = vec![0; u32::from_le_bytes(length) as usize];
    decoder.read_exact(&mut metadata).unwrap();
    let mut recipe: serde_json::Value = serde_json::from_slice(&metadata).unwrap();
    let original = recipe["folders"].as_array().unwrap().clone();
    let mut folders = Vec::new();
    for index in 0..8 {
        for folder in &original {
            let mut folder = folder.clone();
            for range in folder["ranges"].as_array_mut().unwrap() {
                range[0] =
                    serde_json::json!(range[0].as_u64().unwrap() + (index * base.len()) as u64);
            }
            folders.push(folder);
        }
    }
    let mut package = base.repeat(8);
    recipe["size"] = serde_json::json!(package.len());
    recipe["folders"] = serde_json::json!(folders);
    let metadata = serde_json::to_vec(&recipe).unwrap();
    let storage = fixture.context.runtimeStorageHost.as_ref().unwrap();
    storage
        .writeBytes("runtime/temp/application_updates/test/base.apk", &package)
        .unwrap();
    let vfs = fixture.context.vfs().unwrap();
    let expanded = vfs
        .file("/operitapp/data/temp/application_updates/test/expanded")
        .unwrap();
    let restored = vfs
        .file("/operitapp/data/temp/application_updates/test/restored")
        .unwrap();
    let host = fixture.context.systemOperationHost.as_ref().unwrap();
    host.transformApplicationUpdate(
        &fixture.base,
        &expanded,
        ApplicationUpdateTransform::ExpandMsiCabinets {
            recipe: metadata.clone(),
        },
        &HttpDownloadControl::new(),
        &|_, _| {},
    )
    .unwrap();
    host.transformApplicationUpdate(
        &expanded,
        &restored,
        ApplicationUpdateTransform::RestoreMsiCabinets,
        &HttpDownloadControl::new(),
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(restored.readBytes().unwrap(), package);
    storage
        .delete("runtime/temp/application_updates/test/expanded", false)
        .unwrap();
    storage
        .delete("runtime/temp/application_updates/test/restored", false)
        .unwrap();
    let first = &recipe["folders"][0]["ranges"][0];
    let start = first[0].as_u64().unwrap() as usize;
    let length = first[1].as_u64().unwrap() as usize;
    package[start..start + length].fill(0);
    storage
        .writeBytes("runtime/temp/application_updates/test/base.apk", &package)
        .unwrap();
    assert!(host
        .transformApplicationUpdate(
            &fixture.base,
            &expanded,
            ApplicationUpdateTransform::ExpandMsiCabinets { recipe: metadata },
            &HttpDownloadControl::new(),
            &|_, _| {},
        )
        .is_err());
    let path = FileSource::nativePath(&fixture.base).unwrap();
    assert_eq!(
        std::fs::read_dir(Path::new(&path).parent().unwrap())
            .unwrap()
            .count(),
        2,
        "failed workers must remove temporary cabinets and unpublished output"
    );
}

/// Profiles local release MSIs without installing, modifying or publishing either input package.
#[test]
#[ignore = "requires explicitly supplied local release MSI files and a generated manifest"]
fn msi_real_release_round_trip() {
    let mut fixture = fixture();
    fixture.manifest = serde_json::from_slice(
        &std::fs::read(std::env::var("OPERIT_MSI_TEST_MANIFEST").unwrap()).unwrap(),
    )
    .unwrap();
    std::fs::copy(
        std::env::var("OPERIT_MSI_TEST_BASE").unwrap(),
        FileSource::nativePath(&fixture.base).unwrap(),
    )
    .unwrap();
    std::fs::copy(
        std::env::var("OPERIT_MSI_TEST_PATCH").unwrap(),
        FileSource::nativePath(&fixture.patch).unwrap(),
    )
    .unwrap();
    fixture.apply(&HttpDownloadControl::new()).unwrap();
    assert_eq!(
        fixture.output(),
        std::fs::read(std::env::var("OPERIT_MSI_TEST_TARGET").unwrap()).unwrap()
    );
}
