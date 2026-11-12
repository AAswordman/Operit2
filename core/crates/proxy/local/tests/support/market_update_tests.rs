use super::*;
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
    GrepCodeResult, HostEnvironmentDescriptor, HostError,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSourceType;
use operit_plugin_sdk::toolpkg::ToolPkgProtection;
use operit_store::ExtensionStore::ExtensionStore;
use sha2::{Digest, Sha256};
use std::io::Write;

#[derive(Default)]
struct UpdateFailure {
    path: String,
    fail_move: bool,
    fail_read_after_move: bool,
    moved: bool,
}

#[derive(Default)]
struct FaultFileSystem {
    inner: PosixFileSystemHost,
    failure: Mutex<UpdateFailure>,
}

macro_rules! delegate_filesystem {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(#[allow(non_snake_case)]
        fn $name(&self, $($arg: $ty),*) -> $result {
            self.inner.$name($($arg),*)
        })*
    };
}

impl FileSystemHost for FaultFileSystem {
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        let mut failure = self.failure.lock().unwrap();
        if path == failure.path && failure.moved && failure.fail_read_after_move {
            failure.fail_read_after_move = false;
            return Err(HostError::new(
                "injected read failure after market archive replacement",
            ));
        }
        self.inner.readFileBytes(path)
    }
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()> {
        let mut failure = self.failure.lock().unwrap();
        if destination == failure.path && failure.fail_move {
            failure.fail_move = false;
            return Err(HostError::new("injected market archive move failure"));
        }
        self.inner.moveFile(source, destination)?;
        if destination == failure.path {
            failure.moved = true;
        }
        Ok(())
    }
    delegate_filesystem! {
        envLabel() -> &str;
        environmentDescriptor() -> HostEnvironmentDescriptor;
        validatePath(path: &str, paramName: &str) -> HostResult<()>;
        listFiles(path: &str) -> HostResult<Vec<FileEntry>>;
        readFile(path: &str) -> HostResult<String>;
        readFileWithLimit(path: &str, maxBytes: usize) -> HostResult<String>;
        writeFile(path: &str, content: &str, append: bool) -> HostResult<()>;
        writeFileBytes(path: &str, content: &[u8]) -> HostResult<()>;
        deleteFile(path: &str, recursive: bool) -> HostResult<()>;
        fileExists(path: &str) -> HostResult<FileExistence>;
        copyFile(source: &str, destination: &str, recursive: bool) -> HostResult<()>;
        makeDirectory(path: &str, createParents: bool) -> HostResult<()>;
        findFiles(request: FindFilesRequest) -> HostResult<Vec<String>>;
        fileInfo(path: &str) -> HostResult<FileInfo>;
        grepCode(request: GrepCodeRequest) -> HostResult<GrepCodeResult>;
        zipFiles(source: &str, destination: &str) -> HostResult<()>;
        unzipFiles(source: &str, destination: &str) -> HostResult<()>;
        openFile(path: &str) -> HostResult<()>;
        shareFile(path: &str, title: &str) -> HostResult<()>;
    }
}

fn archive(version: &str, publisher: &str) -> Vec<u8> {
    let manifest = json!({
        "schema_version": 1, "toolpkg_id": "dev.test.market-update", "api_version": "2.0.0",
        "version": version, "author": [publisher], "display_name": {"en":"Market update test"},
        "market_origin": {"market":"https://market.example", "toolpkgId":"dev.test.market-update", "version":version, "author":[publisher]},
        "subpackages": []
    });
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(manifest.to_string().as_bytes()).unwrap();
    ToolPkgProtection::wrapMarketArchive(&zip.finish().unwrap().into_inner()).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authenticated_market_updates_preserve_owner_scope_configuration_and_enabled_state() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    verify_market_update(false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_market_archives_update_in_place_without_losing_their_scope() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    verify_market_update(true);
}

fn application(
    filesystem: Arc<dyn FileSystemHost>,
) -> (Arc<TestRuntimeStorageHost>, OperitApplication) {
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(filesystem);
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    (storage, OperitApplication::newWithContext(hosts))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_manifest_origin_alone_cannot_authorize_market_replacement() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let (_storage, app) = application(Arc::new(FaultFileSystem::default()));
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let original = ToolPkgProtection::unwrapMarketArchive(&archive("1.0.0", "publisher")).unwrap();
    let path = manager
        .prepareMarketArtifactDownload("ordinary.toolpkg")
        .unwrap();
    assert!(path.starts_with("/operitapp/data/cache/toolpkg/"));
    let vfs = (&app.hostManager).vfs().unwrap();
    vfs.writeFileBytes(&path, &original).unwrap();
    let result = manager.addPackageFileFromExternalStorage(&path);
    assert!(result.starts_with("Successfully"), "{result}");
    let runtime = manager
        .getToolPkgContainerRuntime("dev.test.market-update")
        .unwrap();
    assert_eq!(runtime.sourceType, ToolPkgSourceType::EXTERNAL);
    let incoming = archive("2.0.0", "publisher");
    let result = manager.addMarketArtifactBytes(
        incoming.clone(),
        "plugin.toolpkg".into(),
        format!("{:x}", Sha256::digest(&incoming)),
    );
    assert!(!result.starts_with("Successfully"), "{result}");
    assert_eq!(vfs.readFileBytes(&runtime.sourcePath).unwrap(), original);
    assert_eq!(
        manager
            .getToolPkgContainerRuntime("dev.test.market-update")
            .unwrap()
            .version,
        "1.0.0"
    );
}

fn verify_market_update(legacy: bool) {
    let filesystem = Arc::new(FaultFileSystem::default());
    let (storage, app) = application(filesystem.clone());
    let vfs = (&app.hostManager).vfs().unwrap();
    let packages = app.packageManager();
    let mut manager = packages.lock().unwrap();
    manager.loadAvailablePackages();
    let name = "dev.test.market-update";
    let first = archive("1.0.0", "publisher");
    let result = if legacy {
        let path = manager
            .prepareMarketArtifactDownload("plugin.toolpkg")
            .unwrap();
        let vfs = (&app.hostManager).vfs().unwrap();
        vfs.writeFileBytes(&path, &first).unwrap();
        // Reproduce the old release's generic import route exactly.
        manager.addPackageFileFromExternalStorage(&path)
    } else {
        manager.addMarketArtifactBytes(
            first.clone(),
            "plugin.toolpkg".into(),
            format!("{:x}", Sha256::digest(&first)),
        )
    };
    assert!(result.starts_with("Successfully"), "{result}");
    assert_eq!(
        manager.getToolPkgContainerRuntime(name).unwrap().sourceType,
        if legacy {
            ToolPkgSourceType::EXTERNAL
        } else {
            ToolPkgSourceType::MARKET
        }
    );
    manager.setPackageScope(name, "space").unwrap();
    manager.setEnabledPackageNames(&[name.into()]).unwrap();
    let store = ExtensionStore::new(storage.clone());
    let config = format!("{}/keep.json", store.configPath(name).unwrap());
    storage.writeBytes(&config, br#"{"value":"keep"}"#).unwrap();
    let before = store.record("package", name).unwrap();
    let old_runtime = manager.getToolPkgContainerRuntime(name).unwrap();
    let next = archive("2.0.0", "publisher");
    let result = manager.addMarketArtifactBytes(
        next.clone(),
        "plugin.toolpkg".into(),
        format!("{:x}", Sha256::digest(&next)),
    );
    assert!(result.starts_with("Successfully"), "{result}");
    let runtime = manager.getToolPkgContainerRuntime(name).unwrap();
    assert_eq!(runtime.version, "2.0.0");
    assert_eq!(runtime.sourceType, ToolPkgSourceType::MARKET);
    assert_eq!(runtime.sourcePath, old_runtime.sourcePath);
    let after = store.record("package", name).unwrap();
    assert_eq!(after.scope, "space");
    assert_eq!(after.sourceName, before.sourceName);
    assert_eq!(after.settings, before.settings);
    assert_eq!(storage.readBytes(&config).unwrap(), br#"{"value":"keep"}"#);
    assert!(manager.getEnabledPackageNames().contains(&name.into()));
    // A failed move (after deleting the old destination), or a failed reload
    // after the swap, must restore both the old archive and installation state.
    let before_archive = vfs.readFileBytes(&runtime.sourcePath).unwrap();
    for fail_move in [true, false] {
        *filesystem.failure.lock().unwrap() = UpdateFailure {
            path: operit_host_api::FileSource::nativePath(&vfs.file(&runtime.sourcePath).unwrap()).unwrap(),
            fail_move,
            fail_read_after_move: !fail_move,
            moved: false,
        };
        let failed = archive("3.0.0", "publisher");
        let result = manager.addMarketArtifactBytes(
            failed.clone(),
            "plugin.toolpkg".into(),
            format!("{:x}", Sha256::digest(&failed)),
        );
        assert!(result.contains("injected"), "{result}");
        assert!(!result.starts_with("Successfully"), "{result}");
        assert_eq!(vfs.readFileBytes(&runtime.sourcePath).unwrap(), before_archive);
        assert_eq!(
            manager.getToolPkgContainerRuntime(name).unwrap().version,
            "2.0.0"
        );
        assert_eq!(
            store.record("package", name).unwrap().settings,
            after.settings
        );
        assert_eq!(store.record("package", name).unwrap().scope, "space");
        assert_eq!(storage.readBytes(&config).unwrap(), br#"{"value":"keep"}"#);
        assert!(manager.getEnabledPackageNames().contains(&name.into()));
    }
    // Another publisher with the same ID cannot replace the owned installation.
    let before_archive = manager.readToolPkgArchiveBytes(name).unwrap();
    let rejected = archive("3.0.0", "other-publisher");
    let result = manager.addMarketArtifactBytes(
        rejected.clone(),
        "plugin.toolpkg".into(),
        format!("{:x}", Sha256::digest(&rejected)),
    );
    assert!(!result.starts_with("Successfully"), "{result}");
    assert_eq!(
        manager.readToolPkgArchiveBytes(name).unwrap(),
        before_archive
    );
    manager.loadAvailablePackages();
    assert_eq!(
        manager.getToolPkgContainerRuntime(name).unwrap().version,
        "2.0.0"
    );
    // Disabled catalog metadata must retain authenticated publisher identity for updates.
    manager.disablePackage(name);
    manager.loadAvailablePackages();
    let disabledSettings = store.record("package", name).unwrap().settings;
    let update = archive("4.0.0", "publisher");
    let result = manager.addMarketArtifactBytes(
        update.clone(), "plugin.toolpkg".into(), format!("{:x}", Sha256::digest(&update)),
    );
    assert!(result.starts_with("Successfully"), "{result}");
    assert_eq!(manager.getToolPkgContainerRuntime(name).unwrap().version, "4.0.0");
    assert!(!manager.isPackageEnabled(name), "updating a disabled installation must preserve disablement");
    assert_eq!(store.record("package", name).unwrap().settings, disabledSettings);
    assert_eq!(storage.readBytes(&config).unwrap(), br#"{"value":"keep"}"#);
}

fn script(version: &str, publisher: &str) -> Vec<u8> {
    let metadata = json!({"name":"dev.test.market-script", "description":version,
        "tools":[{"name":"probe", "description":"probe"}]});
    let source =
        format!("/* METADATA {metadata} */\nexports.probe = function() {{ return '{version}'; }};");
    let origin = operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgMarketOrigin {
        market: "Operit".into(),
        toolpkgId: "dev.test.market-script".into(),
        version: version.into(),
        author: vec![publisher.into()],
    };
    ToolPkgProtection::processArtifactNamedBytesWithMarketOrigin(
        source.as_bytes(),
        "script.js",
        false,
        &origin,
        false,
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn standalone_market_scripts_update_without_uninstalling_or_resetting_configuration() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    for extension in ["js", "ts"] {
        let filesystem = Arc::new(FaultFileSystem::default());
        let (storage, app) = application(filesystem.clone());
        let vfs = (&app.hostManager).vfs().unwrap();
        let packages = app.packageManager();
        let mut manager = packages.lock().unwrap();
        manager.loadAvailablePackages();
        let name = "dev.test.market-script";
        let first = script("1.0.0", "publisher");
        let result = manager.addMarketArtifactBytes(
            first.clone(),
            format!("old.{extension}"),
            format!("{:x}", Sha256::digest(&first)),
        );
        assert!(result.starts_with("Successfully"), "{result}");
        manager.setPackageScope(name, "space").unwrap();
        manager.setEnabledPackageNames(&[name.into()]).unwrap();
        let store = ExtensionStore::new(storage.clone());
        let config = format!("{}/keep.json", store.configPath(name).unwrap());
        storage
            .writeBytes(&config, b"keep-script-settings")
            .unwrap();
        let record = store.record("package", name).unwrap();
        let path = operit_store::RuntimeStorePaths::RuntimeStorePaths::default()
            .runtime_storage_path(&format!(
                "{}/{}",
                ExtensionStore::root("package", &record.scope).unwrap(),
                record.sourceName
            ));
        let next = script("2.0.0", "publisher");
        let result = manager.addMarketArtifactBytes(
            next.clone(),
            format!("new.{extension}"),
            format!("{:x}", Sha256::digest(&next)),
        );
        assert!(result.starts_with("Successfully"), "{result}");
        assert_eq!(vfs.readFileBytes(&path.to_string_lossy()).unwrap(), next);
        assert!(manager.getExecutableAvailablePackages()[name].tools[0]
            .script
            .contains("2.0.0"));
        assert_eq!(
            store.record("package", name).unwrap().settings,
            record.settings
        );
        assert_eq!(
            store.record("package", name).unwrap().sourceName,
            record.sourceName
        );
        for fail_move in [true, false] {
            *filesystem.failure.lock().unwrap() = UpdateFailure {
                path: operit_host_api::FileSource::nativePath(&vfs.file(&path.to_string_lossy()).unwrap()).unwrap(),
                fail_move,
                fail_read_after_move: !fail_move,
                moved: false,
            };
            let incoming = script("3.0.0", "publisher");
            let result = manager.addMarketArtifactBytes(
                incoming.clone(),
                format!("new.{extension}"),
                format!("{:x}", Sha256::digest(&incoming)),
            );
            assert!(result.contains("injected"), "{result}");
            assert_eq!(vfs.readFileBytes(&path.to_string_lossy()).unwrap(), next);
            assert!(manager.getExecutableAvailablePackages()[name].tools[0]
                .script
                .contains("2.0.0"));
            assert_eq!(
                store.record("package", name).unwrap().settings,
                record.settings
            );
            assert_eq!(store.record("package", name).unwrap().scope, "space");
            assert_eq!(storage.readBytes(&config).unwrap(), b"keep-script-settings");
            assert!(manager.getEnabledPackageNames().contains(&name.into()));
        }
        let incoming = script("3.0.0", "other-publisher");
        let result = manager.addMarketArtifactBytes(
            incoming.clone(),
            format!("new.{extension}"),
            format!("{:x}", Sha256::digest(&incoming)),
        );
        assert!(!result.starts_with("Successfully"), "{result}");
        assert_eq!(vfs.readFileBytes(&path.to_string_lossy()).unwrap(), next);
    }
}
