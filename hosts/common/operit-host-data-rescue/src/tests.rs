use super::*;
use operit_host_api::HostResult;
use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};

/// Owns a unique test directory and removes only that exact directory after handles close.
struct Fixture(PathBuf);
impl Fixture {
    /// Creates a private, unique application data tree for one test.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("operit-rescue-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }

    /// Creates one raw payload without using any database or normal bootstrap service.
    fn write(&self, path: &str, bytes: &[u8]) {
        let target = self.0.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }

    /// Returns explicit complete storage roots for this application.
    fn roots(&self) -> RescueRoots {
        RescueRoots { runtime_root: self.0.join("runtime"), workspace_root: self.0.join("workspaces"), cli_config_root: None }
    }

    /// Opens and configures the independent maintenance service.
    fn session(&self, secrets: Option<Arc<dyn HostSecretStore>>) -> NativeRescueSession {
        self.session_for(self.roots(), secrets)
    }

    /// Configures the production session with one explicit physical storage layout.
    fn session_for(&self, roots: RescueRoots, secrets: Option<Arc<dyn HostSecretStore>>) -> NativeRescueSession {
        let mut session = NativeRescueSession::open(self.0.join("runtime"), secrets).unwrap();
        session.execute(json!({"operation": "configure", "roots": roots})).unwrap();
        session
    }

    /// Selects shared or nested runtime and workspace roots for regression coverage.
    fn layout(&self, runtime: &str, workspaces: &str) -> RescueRoots {
        RescueRoots { runtime_root: self.0.join(runtime), workspace_root: self.0.join(workspaces), cli_config_root: None }
    }
}
impl Drop for Fixture {
    /// Deletes only the UUID-owned fixture after its borrowed sessions have been dropped.
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(fs::canonicalize(std::env::temp_dir()).unwrap().as_path()));
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Default)]
struct Secrets { values: Mutex<BTreeMap<String, Vec<u8>>>, fail_writes: AtomicBool }
impl HostSecretStore for Secrets {
    /// Reads a deterministic portable secret for archive verification.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> { Ok(self.values.lock().unwrap().get(key).cloned()) }
    /// Supports deliberate interruption after filesystem publication for recovery tests.
    fn writeSecret(&self, key: &str, value: &[u8]) -> HostResult<()> {
        if self.fail_writes.load(Ordering::SeqCst) && key == SECRET_KEYS[0] { return Err(operit_host_api::HostError::new("injected secret write interruption")); }
        self.values.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    /// Deletes only the exact registered secret requested by the engine.
    fn deleteSecret(&self, key: &str) -> HostResult<()> { self.values.lock().unwrap().remove(key); Ok(()) }
}

/// Streams a portable source archive into another session through the production protocol.
fn transfer(source: &Value, destination: &mut NativeRescueSession) -> String {
    let bytes = fs::read(source["fileReference"].as_str().unwrap()).unwrap();
    let upload = destination.execute(json!({"operation": "beginUpload", "byteLength": bytes.len()})).unwrap();
    let id = upload["archiveId"].as_str().unwrap().to_string();
    for chunk in bytes.chunks(operit_data_rescue::CHUNK_BYTES) {
        destination.execute(json!({"operation": "uploadChunk", "archiveId": id, "bytes": STANDARD.encode(chunk)})).unwrap();
    }
    destination.execute(json!({"operation": "sealUpload", "archiveId": id})).unwrap();
    id
}

/// Verifies complete portable storage and preserves the displaced live data.
#[test]
fn plaintext_roundtrip_preserves_identities_workspaces_secrets_and_originals() {
    let source = Fixture::new();
    source.write("runtime/identities/a/corrupt.sqlite", b"not a sqlite database\0");
    source.write("runtime/identities/b/attachment.bin", &vec![23; 140_000]);
    source.write("workspaces/identities/b/project/empty", b"");
    source.write("client/link/local_runtime_storage.json", br#"{"confirmed":true,"runtimeRoot":"old","workspaceRoot":"old","identities":[]}"#);
    source.write("settings/extra.json", b"raw extra app data");
    fs::create_dir_all(source.0.join("runtime/identities/a/empty-directory/nested")).unwrap();
    fs::create_dir_all(source.0.join("empty-application-directory/nested")).unwrap();
    let secrets = Arc::new(Secrets::default());
    secrets.values.lock().unwrap().insert(SECRET_KEYS[0].into(), b"portable key bytes".to_vec());
    let mut exporting = source.session(Some(secrets));
    let archive = exporting.execute(json!({"operation":"export"})).unwrap();

    let target = Fixture::new();
    target.write("runtime/original", b"old runtime");
    target.write("obsolete-settings", b"must be retained, not mixed into restored data");
    let imported_secrets = Arc::new(Secrets::default());
    let mut importing = target.session(Some(imported_secrets.clone()));
    let id = transfer(&archive, &mut importing);
    assert_eq!(fs::read(target.0.join("runtime/original")).unwrap(), b"old runtime");
    let inspected = importing.execute(json!({"operation":"inspect","archiveId":id})).unwrap();
    assert_eq!(inspected["layout"], "operit.files.v1");
    let restored = importing.execute(json!({"operation":"restore","archiveId":id})).unwrap();
    assert_eq!(restored["restored"], true);
    assert_eq!(fs::read(target.0.join("runtime/identities/a/corrupt.sqlite")).unwrap(), b"not a sqlite database\0");
    assert_eq!(fs::metadata(target.0.join("workspaces/identities/b/project/empty")).unwrap().len(), 0);
    assert!(target.0.join("runtime/identities/a/empty-directory/nested").is_dir());
    assert!(target.0.join("empty-application-directory/nested").is_dir());
    assert!(!target.0.join("obsolete-settings").exists());
    assert_eq!(imported_secrets.readSecret(SECRET_KEYS[0]).unwrap().unwrap(), b"portable key bytes");
    let bootstrap: Value = serde_json::from_slice(&fs::read(target.0.join("client/link/local_runtime_storage.json")).unwrap()).unwrap();
    assert_eq!(bootstrap["runtimeRoot"], json!(target.0.join("runtime")));
    let retained = restored["retainedData"].as_array().unwrap();
    assert!(retained.iter().any(|path| Path::new(path.as_str().unwrap()).join("original").is_file()));
    assert!(!target.0.join("data-rescue/commit.json").exists());
    let completed = fs::read(restored["retainedCatalog"].as_str().unwrap()).unwrap();
    assert!(!completed.windows(b"portable key bytes".len()).any(|part| part == b"portable key bytes"));
    assert!(!completed.windows(STANDARD.encode(b"portable key bytes").len()).any(|part| part == STANDARD.encode(b"portable key bytes").as_bytes()));
}

/// Excludes real identity staging from payloads and metadata while preserving user directories.
#[test]
fn export_excludes_host_archive_namespaces_from_the_complete_runtime_tree() {
    use operit_host_api::ArchiveStagingHost;
    let fixture = Fixture::new();
    fixture.write("runtime/identities/identity-test/data/record", b"user data");
    fixture.write("runtime/identities/identity-test/data/archive_staging/record", b"user-owned directory");
    fixture.write("workspaces/project/archive_staging/record", b"workspace data");
    let active = NativeArchiveStagingHost::new(fixture.0.join("runtime/identities/identity-test"));
    active.createExportArchive("previous").unwrap();
    active.writeExportArchive("previous", 0, &vec![37; 2 * 1024]).unwrap();
    active.sealArchive("previous").unwrap();
    fixture.write("runtime/identities/archive_staging/removed-identity/interrupted.sealed", b"abandoned archive");
    fixture.write("runtime/archive_staging/legacy/old.upload", b"legacy partial archive");
    let mut session = fixture.session(None);
    assert!(!fixture.0.join("runtime/identities/archive_staging/removed-identity/interrupted.sealed").exists());
    assert!(!fixture.0.join("runtime/archive_staging/legacy/old.upload").exists());
    assert!(Path::new(&active.archiveFileReference("previous").unwrap()).exists());
    let archive = session.execute(json!({"operation":"export"})).unwrap();
    let manifest = session.execute(json!({"operation":"inspect", "archiveId":archive["archiveId"]})).unwrap();
    let paths: BTreeSet<_> = manifest["files"].as_array().unwrap().iter()
        .map(|file| file["path"].as_str().unwrap()).collect();
    assert!(paths.contains("runtime/identities/identity-test/data/record"));
    assert!(paths.contains("runtime/identities/identity-test/data/archive_staging/record"));
    assert!(paths.contains("workspaces/project/archive_staging/record"));
    for path in paths.iter().copied().chain(manifest["metadata"]["storageNodes"].as_array().unwrap().iter()
        .map(|node| node["path"].as_str().unwrap())) {
        assert!(!path.starts_with("runtime/identities/archive_staging"), "{path}");
        assert!(!path.starts_with("runtime/archive_staging"), "{path}");
    }
    let exported = PathBuf::from(archive["fileReference"].as_str().unwrap());
    drop(session);
    assert!(!exported.exists());
    drop(active);
}

/// Rejects changed published payloads on resume while retaining every displaced original.
#[test]
fn interrupted_import_rechecks_payloads_before_resuming() {
    let source = Fixture::new();
    source.write("runtime/new", b"verified payload");
    let source_secrets = Arc::new(Secrets::default());
    source_secrets.values.lock().unwrap().insert(SECRET_KEYS[0].into(), b"verified secret".to_vec());
    let mut exporting = source.session(Some(source_secrets));
    let archive = exporting.execute(json!({"operation":"export"})).unwrap();
    let target = Fixture::new();
    target.write("runtime/old", b"untouched original");
    let secrets = Arc::new(Secrets::default());
    secrets.fail_writes.store(true, Ordering::SeqCst);
    let mut importing = target.session(Some(secrets.clone()));
    let id = transfer(&archive, &mut importing);
    assert!(importing.execute(json!({"operation":"restore","archiveId":id})).is_err());
    let journal: Journal = serde_json::from_slice(&fs::read(target.0.join("data-rescue/commit.json")).unwrap()).unwrap();
    let retained = journal.publications.iter().find(|plan| plan.target == target.0.join("runtime")).unwrap().retained.clone();
    target.write("runtime/new", b"tampered payload");
    secrets.fail_writes.store(false, Ordering::SeqCst);
    let error = importing.execute(json!({"operation":"resumeRestore"})).unwrap_err();
    assert!(error.starts_with("Pending import payload integrity check failed:"), "{error}");
    assert_eq!(fs::read(retained.join("old")).unwrap(), b"untouched original");
    assert!(target.0.join("data-rescue/commit.json").is_file());
    assert!(secrets.readSecret(SECRET_KEYS[0]).unwrap().is_none());
}

/// Rejects incomplete and aliased directory metadata before creating any staging tree.
#[test]
fn native_metadata_rejects_aliases_and_missing_ancestors() {
    let fixture = Fixture::new();
    fixture.write("runtime/original", b"must remain unchanged");
    let host = NativeRescueHost::new(&fixture.0.join("runtime"), fixture.roots(), None).unwrap();
    let payload = RescueFile { path: "runtime/folder/file".into(), byte_length: 0, sha256: "0".repeat(64) };
    let mut manifest = RescueManifest {
        format: "operit.data-rescue".into(), version: 1, layout: "operit.files.v1".into(),
        files: vec![payload, RescueFile { path: SYSTEM_SECRETS.into(), byte_length: 2, sha256: "0".repeat(64) }],
        metadata: json!({"storageNodes": [
            {"path":"runtime", "directory":true, "unixMode":null},
            {"path":"runtime/folder/file", "directory":false, "unixMode":null}
        ]}),
    };
    assert!(host.begin_restore(&uuid::Uuid::new_v4().to_string(), &manifest).unwrap_err().starts_with("Missing directory metadata:"));
    manifest.metadata["storageNodes"].as_array_mut().unwrap().extend([
        json!({"path":"runtime/folder", "directory":true, "unixMode":null}),
        json!({"path":"runtime/FOLDER", "directory":true, "unixMode":null}),
    ]);
    assert!(host.begin_restore(&uuid::Uuid::new_v4().to_string(), &manifest).unwrap_err().starts_with("Aliased or duplicate native destination:"));
    assert_eq!(fs::read(fixture.0.join("runtime/original")).unwrap(), b"must remain unchanged");
    assert!(!fixture.0.join("data-rescue").exists());
}

/// Exports a broken bootstrap without decoding it or opening any database.
#[test]
fn damaged_bootstrap_is_available_as_raw_data() {
    let fixture = Fixture::new();
    fixture.write("client/link/local_runtime_storage.json", b"{broken startup record");
    let mut session = fixture.session(None);
    let startup = session.describe_startup().unwrap();
    assert_eq!(STANDARD.decode(startup["bootstrapBase64"].as_str().unwrap()).unwrap(), b"{broken startup record");
    let archive = session.execute(json!({"operation":"export"})).unwrap();
    assert!(session.execute(json!({"operation":"inspect","archiveId":archive["archiveId"]})).is_ok());
}

/// Blocks concurrent runtime access and resumes a journal independently of damaged bootstrap.
#[test]
fn interrupted_publication_requires_explicit_resume_and_keeps_originals() {
    let source = Fixture::new();
    source.write("runtime/new", b"new data");
    let exporting_secrets = Arc::new(Secrets::default());
    exporting_secrets.values.lock().unwrap().insert(SECRET_KEYS[0].into(), b"imported key".to_vec());
    let mut exporting = source.session(Some(exporting_secrets));
    let archive = exporting.execute(json!({"operation":"export"})).unwrap();
    let target = Fixture::new();
    target.write("runtime/old", b"original data");
    let secrets = Arc::new(Secrets::default());
    secrets.fail_writes.store(true, Ordering::SeqCst);
    {
        let mut importing = target.session(Some(secrets.clone()));
        assert!(StorageLease::acquire(&target.0.join("runtime"), false).is_err());
        let id = transfer(&archive, &mut importing);
        let error = importing.execute(json!({"operation":"restore","archiveId":id})).unwrap_err();
        assert!(error.starts_with("injected secret write interruption"));
    }
    assert!(StorageLease::acquire(&target.0.join("runtime"), false).is_err());
    target.write("client/link/local_runtime_storage.json", b"broken after interruption");
    secrets.fail_writes.store(false, Ordering::SeqCst);
    {
        let mut importing = NativeRescueSession::open(target.0.join("runtime"), Some(secrets)).unwrap();
        let description = importing.execute(json!({"operation": "configureAutomatic"})).unwrap();
        assert_eq!(description["roots"]["runtime"], json!(target.0.join("runtime")));
        assert_eq!(description["roots"]["workspaces"], json!(target.0.join("workspaces")));
        let restored = importing.execute(json!({"operation":"resumeRestore"})).unwrap();
        assert_eq!(restored["restored"], true);
        assert!(restored["retainedData"].as_array().unwrap().iter().any(|path| Path::new(path.as_str().unwrap()).join("old").is_file()));
    }
    assert!(StorageLease::acquire(&target.0.join("runtime"), false).is_ok());
}

/// Creates the directory aliases used by Unix application containers.
#[cfg(unix)]
fn directory_alias(target: &Path, alias: &Path) {
    std::os::unix::fs::symlink(target, alias).unwrap();
}

/// Creates a Windows directory junction without requiring symbolic-link privileges.
#[cfg(windows)]
fn directory_alias(target: &Path, alias: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(alias)
        .arg(target)
        .output()
        .unwrap();
    assert!(output.status.success(), "Cannot create test junction: {}", String::from_utf8_lossy(&output.stderr));
}

/// Resolves fresh-install destinations without creating them and rejects invalid directory roots.
#[test]
fn physical_roots_preserve_missing_directories() {
    let fixture = Fixture::new();
    assert_eq!(absolute_root(&fixture.0).unwrap(), fixture.0);
    for relative in ["runtime", "not-created/identities/default/runtime"] {
        let root = fixture.0.join(relative);
        assert_eq!(absolute_root(&root).unwrap(), root);
        assert!(!root.exists());
    }
    assert!(absolute_root(Path::new("relative/runtime")).is_err());
    let mut traversal = fixture.0.as_os_str().to_os_string();
    traversal.push(format!("{separator}..{separator}escape", separator = std::path::MAIN_SEPARATOR));
    assert!(absolute_root(Path::new(&traversal)).is_err());
    fixture.write("regular-file", b"not a directory");
    assert!(absolute_root(&fixture.0.join("regular-file")).is_err());
    assert!(absolute_root(&fixture.0.join("regular-file/runtime")).is_err());
}

/// Shares one storage lock across ancestor aliases before the runtime directory exists.
#[test]
fn ancestor_aliases_share_runtime_and_maintenance_leases() {
    let fixture = Fixture::new();
    let physical = fixture.0.join("physical");
    let application = physical.join("app/files");
    fs::create_dir_all(&application).unwrap();
    let alias = fixture.0.join("user-0");
    directory_alias(&physical, &alias);
    let aliased_runtime = alias.join("app/files/runtime");
    let runtime = application.join("runtime");
    assert!(!runtime.exists());
    assert_eq!(absolute_root(&aliased_runtime).unwrap(), runtime);
    let normal = StorageLease::acquire(&aliased_runtime, false).unwrap();
    assert!(StorageLease::acquire(&runtime, true).is_err());
    drop(normal);
    let mut session = NativeRescueSession::open(aliased_runtime.clone(), None).unwrap();
    assert_eq!(session.default_runtime, runtime);
    let described = session.execute(json!({
        "operation": "configure",
        "roots": RescueRoots {
            runtime_root: aliased_runtime.clone(),
            workspace_root: alias.join("app/files/workspaces"),
            cli_config_root: None,
        },
    })).unwrap();
    assert_eq!(described["applicationRoot"], json!(application));
    assert_eq!(described["roots"]["runtime"], json!(runtime));
    assert!(StorageLease::acquire(&aliased_runtime, false).is_err());
    assert!(StorageLease::acquire(&runtime, false).is_err());
    drop(session);
    assert!(StorageLease::acquire(&runtime, false).is_ok());
    fs::create_dir_all(&runtime).unwrap();
    assert_eq!(absolute_root(&aliased_runtime).unwrap(), runtime);
}

/// Maps ancestor aliases and nested roots into one physical publication tree.
#[test]
fn ancestor_aliases_share_overlapping_publication_roots() {
    let fixture = Fixture::new();
    let physical = fixture.0.join("physical");
    fs::create_dir_all(&physical).unwrap();
    let alias = fixture.0.join("alias");
    directory_alias(&physical, &alias);
    let roots = RescueRoots {
        runtime_root: physical.join("runtime"),
        workspace_root: alias.join("runtime/nested"),
        cli_config_root: None,
    };
    let host = NativeRescueHost::new(&fixture.0.join("application/runtime"), roots, None).unwrap();
    assert_eq!(host.roots["runtime"], physical.join("runtime"));
    assert_eq!(host.roots["workspaces"], physical.join("runtime/nested"));
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(host.staged_path(&id, "runtime/nested/file").unwrap(), host.staged_path(&id, "workspaces/file").unwrap());
    assert_eq!(host.publication_roots.len(), 3);
}

/// Rejects root links and linked descendants even when ancestor aliases are accepted.
#[test]
fn root_links_and_descendant_links_are_rejected() {
    let fixture = Fixture::new();
    let external = fixture.0.join("external");
    fs::create_dir_all(&external).unwrap();
    let linked_root = fixture.0.join("linked-root");
    directory_alias(&external, &linked_root);
    let error = absolute_root(&linked_root).unwrap_err();
    assert!(error.starts_with("Filesystem links are not accepted at the rescue boundary:"));
    let runtime = fixture.0.join("runtime");
    fs::create_dir_all(&runtime).unwrap();
    directory_alias(&external, &runtime.join("linked-directory"));
    assert!(checked_descendant(&runtime, "linked-directory/secret").is_err());
    assert!(collect_files(&runtime, "runtime", &mut Vec::new(), &mut Vec::new(), &NativeRescueHost::new(&fixture.0.join("runtime"), fixture.roots(), None).unwrap()).is_err());
}

/// Reports dangling ancestor aliases without treating their targets as new data directories.
#[test]
fn dangling_ancestor_aliases_are_rejected() {
    let fixture = Fixture::new();
    let missing = fixture.0.join("missing-target");
    let alias = fixture.0.join("dangling-alias");
    directory_alias(&missing, &alias);
    assert!(absolute_root(&alias.join("runtime")).is_err());
    assert!(!missing.exists());
}

/// Restores shared and nested layouts without duplicate writes or overlapping directory moves.
#[test]
fn overlapping_roots_roundtrip_and_remain_portable() {
    for (runtime, workspaces) in [
        ("runtime", "runtime"),
        ("runtime", "runtime/workspaces"),
        ("workspaces/runtime", "workspaces"),
        ("storage/runtime", "storage/workspaces"),
        ("client", "client/workspaces"),
        ("runtime", "secure"),
    ] {
        let source = Fixture::new();
        source.write(&format!("{runtime}/identities/a/data"), &vec![31; 140_000]);
        source.write(&format!("{workspaces}/identities/a/project/file"), b"workspace payload");
        source.write("storage/extra", b"extra application data");
        let source_roots = source.layout(runtime, workspaces);
        source.write("client/link/local_runtime_storage.json", &serde_json::to_vec(&json!({
            "confirmed": true, "runtimeRoot": source_roots.runtime_root,
            "workspaceRoot": source_roots.workspace_root, "identities": [],
        })).unwrap());
        let mut exporting = NativeRescueSession::open(source.0.join("runtime"), None).unwrap();
        exporting.execute(json!({"operation": "configureAutomatic"})).unwrap();
        let archive = exporting.execute(json!({"operation": "export"})).unwrap();
        for (target_runtime, target_workspaces) in [(runtime, workspaces), ("runtime", "workspaces")] {
            let target = Fixture::new();
            target.write(&format!("{target_runtime}/original"), b"original data");
            let roots = target.layout(target_runtime, target_workspaces);
            let mut importing = target.session_for(roots.clone(), None);
            let id = transfer(&archive, &mut importing);
            let restored = importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap();
            assert_eq!(restored["restored"], true);
            assert_eq!(fs::read(roots.runtime_root.join("identities/a/data")).unwrap(), vec![31; 140_000]);
            assert_eq!(fs::read(roots.workspace_root.join("identities/a/project/file")).unwrap(), b"workspace payload");
            assert_eq!(fs::read(target.0.join("storage/extra")).unwrap(), b"extra application data");
            assert!(!roots.runtime_root.join("original").exists());
            let retained: Vec<_> = restored["retainedData"].as_array().unwrap().iter()
                .map(|path| PathBuf::from(path.as_str().unwrap())).collect();
            assert!(retained.iter().any(|root| {
                root.join("original").is_file() || root.join("runtime/original").is_file()
            }));
            let bootstrap: Value = serde_json::from_slice(&fs::read(target.0.join("client/link/local_runtime_storage.json")).unwrap()).unwrap();
            assert_eq!(bootstrap["runtimeRoot"], json!(roots.runtime_root));
            assert_eq!(bootstrap["workspaceRoot"], json!(roots.workspace_root));
            let completed: Journal = serde_json::from_slice(&fs::read(restored["retainedCatalog"].as_str().unwrap()).unwrap()).unwrap();
            for (index, plan) in completed.publications.iter().enumerate() {
                for other in completed.publications.iter().skip(index + 1) {
                    assert!(!plan.target.starts_with(&other.target) && !other.target.starts_with(&plan.target));
                }
            }
        }
    }
}

/// Rejects incompatible payload aliases before staging or changing the original data.
#[test]
fn shared_restore_destinations_reject_conflicting_payloads() {
    let source = Fixture::new();
    source.write("runtime/same", b"runtime content");
    source.write("workspaces/same", b"different workspace content");
    let mut exporting = source.session(None);
    let archive = exporting.execute(json!({"operation": "export"})).unwrap();
    let target = Fixture::new();
    target.write("runtime/original", b"original data");
    let mut importing = target.session_for(target.layout("runtime", "runtime"), None);
    let id = transfer(&archive, &mut importing);
    let error = importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap_err();
    assert!(error.starts_with("Conflicting archive entries share a destination:"), "{error}");
    assert_eq!(fs::read(target.0.join("runtime/original")).unwrap(), b"original data");
    assert!(!target.0.join("runtime/same").exists());
    assert!(!target.0.join("data-rescue/commit.json").exists());
}

/// Publishes external CLI profile roots once when they contain runtime and workspace storage.
#[test]
fn external_cli_profile_roots_share_one_publication_tree() {
    let source = Fixture::new();
    let source_storage = Fixture::new();
    source_storage.write("profile/runtime/data", &vec![41; 140_000]);
    source_storage.write("profile/runtime/workspaces/project/file", b"workspace payload");
    source_storage.write("profile/storage.json", br#"{"runtimeRoot":"old","workspaceRoot":"old"}"#);
    let source_roots = RescueRoots {
        runtime_root: source_storage.0.join("profile/runtime"),
        workspace_root: source_storage.0.join("profile/runtime/workspaces"),
        cli_config_root: Some(source_storage.0.join("profile")),
    };
    let mut exporting = source.session_for(source_roots, None);
    let archive = exporting.execute(json!({"operation": "export"})).unwrap();
    let target = Fixture::new();
    let target_storage = Fixture::new();
    target_storage.write("profile/original", b"original profile");
    let target_roots = RescueRoots {
        runtime_root: target_storage.0.join("profile/runtime"),
        workspace_root: target_storage.0.join("profile/runtime/workspaces"),
        cli_config_root: Some(target_storage.0.join("profile")),
    };
    let mut importing = target.session_for(target_roots.clone(), None);
    let id = transfer(&archive, &mut importing);
    let restored = importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap();
    assert_eq!(fs::read(target_roots.runtime_root.join("data")).unwrap(), vec![41; 140_000]);
    assert_eq!(fs::read(target_roots.workspace_root.join("project/file")).unwrap(), b"workspace payload");
    let profile = target_storage.0.join("profile");
    let config: Value = serde_json::from_slice(&fs::read(profile.join("storage.json")).unwrap()).unwrap();
    assert_eq!(config["runtimeRoot"], json!(target_roots.runtime_root));
    assert_eq!(config["workspaceRoot"], json!(target_roots.workspace_root));
    let completed: Journal = serde_json::from_slice(&fs::read(restored["retainedCatalog"].as_str().unwrap()).unwrap()).unwrap();
    let external: Vec<_> = completed.publications.iter().filter(|plan| plan.target.starts_with(&target_storage.0)).collect();
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].target, profile);
    assert_eq!(fs::read(external[0].retained.join("original")).unwrap(), b"original profile");
}

/// Resumes shared publication trees from recorded roots while retaining their displaced originals.
#[test]
fn overlapping_roots_resume_interrupted_publication() {
    let source = Fixture::new();
    source.write("runtime/new", b"runtime payload");
    source.write("workspaces/project/file", b"workspace payload");
    let exporting_secrets = Arc::new(Secrets::default());
    exporting_secrets.values.lock().unwrap().insert(SECRET_KEYS[0].into(), b"imported key".to_vec());
    let mut exporting = source.session(Some(exporting_secrets));
    let archive = exporting.execute(json!({"operation": "export"})).unwrap();
    for (runtime, workspaces) in [
        ("runtime", "runtime"),
        ("runtime", "runtime/workspaces"),
        ("storage/runtime", "storage/workspaces"),
        ("client", "client/workspaces"),
    ] {
        let target = Fixture::new();
        target.write(&format!("{runtime}/original"), b"original payload");
        let roots = target.layout(runtime, workspaces);
        let secrets = Arc::new(Secrets::default());
        secrets.fail_writes.store(true, Ordering::SeqCst);
        {
            let mut importing = target.session_for(roots.clone(), Some(secrets.clone()));
            let id = transfer(&archive, &mut importing);
            let error = importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap_err();
            assert!(error.starts_with("injected secret write interruption"), "{error}");
        }
        assert!(StorageLease::acquire(&target.0.join("runtime"), false).is_err());
        secrets.fail_writes.store(false, Ordering::SeqCst);
        let mut importing = NativeRescueSession::open(target.0.join("runtime"), Some(secrets.clone())).unwrap();
        let configured = importing.execute(json!({"operation": "configureAutomatic"})).unwrap();
        assert_eq!(configured["roots"]["runtime"], json!(roots.runtime_root));
        assert_eq!(configured["roots"]["workspaces"], json!(roots.workspace_root));
        let restored = importing.execute(json!({"operation": "resumeRestore"})).unwrap();
        assert_eq!(fs::read(roots.runtime_root.join("new")).unwrap(), b"runtime payload");
        assert_eq!(fs::read(roots.workspace_root.join("project/file")).unwrap(), b"workspace payload");
        assert!(restored["retainedData"].as_array().unwrap().iter().any(|path| {
            let retained = PathBuf::from(path.as_str().unwrap());
            retained.join("original").is_file() || retained.join("runtime/original").is_file()
        }));
        assert_eq!(secrets.readSecret(SECRET_KEYS[0]).unwrap().unwrap(), b"imported key");
        drop(importing);
        assert!(StorageLease::acquire(&target.0.join("runtime"), false).is_ok());
    }
}

/// Allows vetted platform prefixes to be links while keeping rescue behavior intact.
#[test]
#[cfg(unix)]
fn platform_symlinked_ancestors_keep_startup_and_maintenance_available() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let application = fixture.0.join("application");
    fs::create_dir_all(application.join("runtime")).unwrap();
    fs::create_dir_all(application.join("workspaces")).unwrap();
    let alias = fixture.0.join("alias");
    symlink(&application, &alias).unwrap();
    if let Err(error) = StorageLease::acquire(&alias.join("runtime"), false) {
        panic!("a symlinked platform prefix must not block startup: {error}");
    }
    let mut session = NativeRescueSession::open(alias.join("runtime"), None).unwrap();
    session.execute(json!({"operation":"configure","roots":{
        "runtimeRoot": alias.join("runtime"),
        "workspaceRoot": alias.join("workspaces")
    }})).unwrap();
    let archive = session.execute(json!({"operation":"export"})).unwrap();
    assert!(Path::new(archive["fileReference"].as_str().unwrap()).is_file());
}

/// Continues to reject a redirected root and link components inside an accepted root.
#[test]
#[cfg(unix)]
fn direct_links_are_still_rejected_at_the_rescue_boundary() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.write("runtime/file", b"payload");
    let linked_root = fixture.0.join("linked-runtime");
    symlink(fixture.0.join("runtime"), &linked_root).unwrap();
    let mut linked = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    let error = linked.execute(json!({"operation":"configure","roots":{
        "runtimeRoot": linked_root.clone(),
        "workspaceRoot": fixture.0.join("workspaces")
    }})).unwrap_err();
    assert!(error.starts_with("Filesystem links are not accepted at the rescue boundary:"), "{error}");
    drop(linked);
    match NativeRescueSession::open(linked_root.clone(), None) {
        Ok(_) => panic!("a redirected default runtime root must be rejected"),
        Err(error) => assert!(error.starts_with("Filesystem links are not accepted at the rescue boundary:"), "{error}"),
    }

    let mut session = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    symlink(&fixture.0, fixture.0.join("runtime/escape")).unwrap();
    let error = session.execute(json!({"operation":"configure","roots":fixture.roots()})).unwrap_err();
    assert!(error.starts_with("Filesystem links require an explicit archival policy:"), "{error}");
}

/// Exports all identity preferences as plaintext and restores them without a backup password.
#[test]
fn rescue_exports_portable_preferences_without_mutating_sources() {
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    for workspaces in ["workspaces", "runtime"] {
        let source = Fixture::new();
        let key = serde_json::to_vec(&json!({
            "format": "operit.preferences.encryption.key", "version": 1,
            "algorithm": "XChaCha20Poly1305", "keyId": "portable-test-key",
            "key": base64::engine::general_purpose::STANDARD_NO_PAD.encode([7u8; 32]),
        })).unwrap();
        let cipher = PreferencesEncryption::decodeKeyBytes(&key).unwrap();
        let storage_path = "runtime/config/preferences/model_configs.preferences.json";
        let plaintext = br#"{"api_key":"user-owned-token","models":[]}"#;
        let encrypted = cipher.encrypt(storage_path, plaintext).unwrap();
        for identity in ["a", "b"] {
            source.write(&format!("runtime/identities/{identity}/config/preferences/model_configs.preferences.json"), &encrypted);
        }
        let secrets = Arc::new(Secrets::default());
        secrets.values.lock().unwrap().insert(SECRET_KEYS[0].into(), key);
        let mut exporting = source.session_for(source.layout("runtime", workspaces), Some(secrets));
        let archive = exporting.execute(json!({"operation": "export"})).unwrap();
        let target = Fixture::new();
        let mut importing = target.session_for(target.layout("runtime", workspaces), Some(Arc::new(Secrets::default())));
        let id = transfer(&archive, &mut importing);
        let manifest = importing.execute(json!({"operation": "inspect", "archiveId": id})).unwrap();
        for identity in ["a", "b"] {
            let path = format!("runtime/identities/{identity}/config/preferences/model_configs.preferences.json");
            let record = manifest["files"].as_array().unwrap().iter().find(|file| file["path"] == path).unwrap();
            assert_eq!(record["byteLength"], plaintext.len());
            assert_eq!(record["sha256"], format!("{:x}", Sha256::digest(plaintext)));
            assert_eq!(fs::read(source.0.join(&path)).unwrap(), encrypted);
        }
        importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap();
        for identity in ["a", "b"] {
            assert_eq!(fs::read(target.0.join(format!("runtime/identities/{identity}/config/preferences/model_configs.preferences.json"))).unwrap(), plaintext);
        }
    }
}

/// Reports the affected preference path when the original encryption key is unavailable.
#[test]
fn rescue_rejects_missing_preference_keys_explicitly() {
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    let source = Fixture::new();
    let key = serde_json::to_vec(&json!({
        "format": "operit.preferences.encryption.key", "version": 1,
        "algorithm": "XChaCha20Poly1305", "keyId": "missing-test-key",
        "key": base64::engine::general_purpose::STANDARD_NO_PAD.encode([8u8; 32]),
    })).unwrap();
    let path = "runtime/config/preferences/model_configs.preferences.json";
    let bytes = PreferencesEncryption::decodeKeyBytes(&key).unwrap().encrypt(path, b"{}").unwrap();
    source.write(path, &bytes);
    let mut session = source.session(Some(Arc::new(Secrets::default())));
    let error = session.execute(json!({"operation": "export"})).unwrap_err();
    assert!(error.contains(path), "{error}");
    assert!(error.ends_with("Preferences encryption key is missing from the Host secret store"), "{error}");
    assert_eq!(fs::read(source.0.join(path)).unwrap(), bytes);
}

/// Uses recorded complete roots without parsing unrelated damaged application state.
#[test]
fn automatic_configuration_uses_recorded_custom_locations() {
    let fixture = Fixture::new();
    fixture.write("selected-runtime/identities/a/data", b"selected runtime");
    fixture.write("selected-workspaces/identities/a/file", b"selected workspace");
    fixture.write("runtime/unselected", b"not selected");
    let config = json!({
        "confirmed": true,
        "runtimeRoot": fixture.0.join("selected-runtime"),
        "workspaceRoot": fixture.0.join("selected-workspaces"),
        "identities": "damaged identity configuration",
    });
    fixture.write("client/link/local_runtime_storage.json", &serde_json::to_vec(&config).unwrap());
    let mut session = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    let description = session.execute(json!({"operation": "configureAutomatic"})).unwrap();
    assert_eq!(description["roots"]["runtime"], config["runtimeRoot"]);
    assert_eq!(description["roots"]["workspaces"], config["workspaceRoot"]);
    let archive = session.execute(json!({"operation": "export"})).unwrap();
    let manifest = session.execute(json!({"operation": "inspect", "archiveId": archive["archiveId"]})).unwrap();
    assert!(manifest["files"].as_array().unwrap().iter().any(|file| file["path"] == "runtime/identities/a/data"));
    assert!(manifest["files"].as_array().unwrap().iter().any(|file| file["path"] == "workspaces/identities/a/file"));
}

/// Reports missing location records without assigning the platform's standard directories.
#[test]
fn automatic_configuration_rejects_missing_location_records() {
    let fixture = Fixture::new();
    fixture.write("runtime/data", b"must not select without a location record");
    let mut session = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    let error = session.execute(json!({"operation": "configureAutomatic"})).unwrap_err();
    assert_eq!(error, "找不到应用的数据位置记录，无法自动定位数据。");
    assert!(session.service.is_none());
}

/// Keeps damaged location records intact and leaves the export service unconfigured.
#[test]
fn automatic_configuration_rejects_damaged_location_records() {
    let fixture = Fixture::new();
    let damaged = b"{damaged location record";
    fixture.write("client/link/local_runtime_storage.json", damaged);
    let mut session = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    let error = session.execute(json!({"operation": "configureAutomatic"})).unwrap_err();
    assert!(error.starts_with("应用的数据位置记录损坏，无法自动定位数据："), "{error}");
    assert!(session.service.is_none());
    assert_eq!(fs::read(fixture.0.join("client/link/local_runtime_storage.json")).unwrap(), damaged);
}

/// Refuses unconfirmed storage settings rather than activating them implicitly.
#[test]
fn automatic_configuration_rejects_unconfirmed_location_records() {
    let fixture = Fixture::new();
    fixture.write("client/link/local_runtime_storage.json", &serde_json::to_vec(&json!({
        "confirmed": false, "runtimeRoot": fixture.0.join("runtime"),
        "workspaceRoot": fixture.0.join("workspaces"),
    })).unwrap());
    let mut session = NativeRescueSession::open(fixture.0.join("runtime"), None).unwrap();
    let error = session.execute(json!({"operation": "configureAutomatic"})).unwrap_err();
    assert_eq!(error, "应用尚未完成数据位置设置，无法自动定位数据。");
    assert!(session.service.is_none());
}

/// Captures shared-engine counters while exercising the production native storage Host.
struct RecordingHost { inner: NativeRescueHost, events: Mutex<Vec<RescueProgress>> }
impl RescueHost for RecordingHost {
    /// Retains a measured event for assertions without changing storage work.
    fn report_progress(&self, progress: RescueProgress) { self.events.lock().unwrap().push(progress); }
    /// Exposes the production native archive layout.
    fn layout(&self) -> &str { self.inner.layout() }
    /// Reads the production native metadata.
    fn describe(&self) -> Result<Value, String> { self.inner.describe() }
    /// Enumerates the fixture through the production Host.
    fn list_files(&self) -> Result<Vec<RescueFile>, String> { self.inner.list_files() }
    /// Reads real bounded source ranges.
    fn read_file(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String> { assert!(length <= operit_data_rescue::CHUNK_BYTES); self.inner.read_file(path, offset, length) }
    /// Converts preferences through the production key contract.
    fn export_preferences(&self, path: &str) -> Result<Option<Vec<u8>>, String> { self.inner.export_preferences(path) }
    /// Prepares a real isolated publication tree.
    fn begin_restore(&self, id: &str, manifest: &RescueManifest) -> Result<(), String> { self.inner.begin_restore(id, manifest) }
    /// Appends one measured restore chunk to real storage.
    fn write_restore(&self, id: &str, path: &str, chunk: &[u8]) -> Result<(), String> { self.inner.write_restore(id, path, chunk) }
    /// Flushes a real staged file.
    fn finish_restore_file(&self, id: &str, path: &str) -> Result<(), String> { self.inner.finish_restore_file(id, path) }
    /// Publishes the complete production journal.
    fn commit_restore(&self, id: &str, manifest: &RescueManifest) -> Result<Value, String> { self.inner.commit_restore(id, manifest) }
    /// Resumes a persisted production journal.
    fn resume_restore(&self) -> Result<Value, String> { self.inner.resume_restore() }
    /// Removes only fixture-owned uncommitted staging.
    fn discard_restore(&self, id: &str) -> Result<(), String> { self.inner.discard_restore(id) }
}

/// Requires exact chunk-based progress for packing, inspection, and extraction, including empty files.
#[test]
fn rescue_progress_tracks_real_payloads_through_export_and_restore() {
    let fixture = Fixture::new();
    fixture.write("runtime/data.bin", &vec![47u8; 196_625]);
    fixture.write("runtime/empty", &[]);
    let host = Arc::new(RecordingHost {
        inner: NativeRescueHost::new(&fixture.0.join("runtime"), fixture.roots(), None).unwrap(),
        events: Mutex::new(Vec::new()),
    });
    let service = DataRescue { host: host.clone(), archives: Arc::new(NativeArchiveStagingHost::new(fixture.0.join("data-rescue/staging"))) };
    let archive = service.export().unwrap();
    let id = archive["archiveId"].as_str().unwrap();
    let manifest = service.inspect(id).unwrap();
    service.restore(id).unwrap();
    for phase in ["export", "verifyArchive", "restore"] {
        let events = host.events.lock().unwrap();
        let counters: Vec<_> = events.iter().filter(|event| event.phase == phase).collect();
        assert!(!counters.is_empty(), "missing {phase}");
        assert_eq!(counters[0].completed, 0);
        assert_eq!(counters.last().unwrap().completed, payload_bytes(&manifest.files).unwrap());
        for counter in &counters {
            assert_eq!(counter.total, Some(payload_bytes(&manifest.files).unwrap()));
            assert_eq!(counter.unit, "bytes");
            assert!(counter.completed <= counter.total.unwrap());
        }
        // A new archive verification pass begins at zero before restore; each pass is measured separately.
        for pair in counters.windows(2).filter(|pair| pair[1].completed != 0) {
            assert!(pair[1].completed >= pair[0].completed);
            assert!(pair[1].completed - pair[0].completed <= operit_data_rescue::CHUNK_BYTES as u64);
        }
    }
    let publication = host.inner.progress.lock().unwrap().clone().unwrap();
    assert_eq!(publication.phase, "publish");
    assert_eq!(Some(publication.completed), publication.total);
}

/// Copies an exported archive through the Host and reports the actual destination byte count.
#[test]
fn rescue_save_progress_matches_written_archive() {
    let fixture = Fixture::new();
    fixture.write("runtime/data.bin", &vec![9u8; 131_081]);
    let mut session = fixture.session(None);
    let archive = session.execute(json!({"operation": "export"})).unwrap();
    let destination = fixture.0.join("saved-backup.zip");
    session.execute(json!({"operation": "saveArchive", "archiveId": archive["archiveId"], "destination": destination})).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), fs::read(archive["fileReference"].as_str().unwrap()).unwrap());
    let progress = session.progress().lock().unwrap().clone().unwrap();
    assert_eq!(progress.phase, "save");
    assert_eq!(Some(progress.completed), progress.total);
    assert_eq!(progress.completed, fs::metadata(destination).unwrap().len());
}

/// Rejects saving onto the staged archive before opening or truncating any output file.
#[test]
fn rescue_save_rejects_the_staged_archive_destination() {
    let fixture = Fixture::new();
    fixture.write("runtime/data", b"original payload");
    let mut session = fixture.session(None);
    let archive = session.execute(json!({"operation": "export"})).unwrap();
    let destination = PathBuf::from(archive["fileReference"].as_str().unwrap());
    let original = fs::read(&destination).unwrap();
    let error = session.execute(json!({
        "operation": "saveArchive", "archiveId": archive["archiveId"], "destination": destination,
    })).unwrap_err();
    assert_eq!(error, "Save destination is the staged archive");
    assert_eq!(fs::read(&destination).unwrap(), original);
}

/// Uses independent file-backed keys for the base runtime and each isolated identity.
#[test]
fn profile_file_keys_export_and_restore_every_identity_without_changing_sources() {
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    let source = Fixture::new();
    let storage_path = "runtime/config/preferences/model_configs.preferences.json";
    let mut originals = Vec::new();
    for (index, profile) in ["runtime", "runtime/identities/a", "runtime/identities/b"].iter().enumerate() {
        let key = serde_json::to_vec(&json!({
            "format": "operit.preferences.encryption.key", "version": 1,
            "algorithm": "XChaCha20Poly1305", "keyId": format!("profile-{index}"),
            "key": base64::engine::general_purpose::STANDARD_NO_PAD.encode([index as u8 + 1; 32]),
        })).unwrap();
        let plaintext = serde_json::to_vec(&json!({"api_key": format!("profile-token-{index}")})).unwrap();
        let encrypted = PreferencesEncryption::decodeKeyBytes(&key).unwrap().encrypt(storage_path, &plaintext).unwrap();
        let path = format!("{profile}/config/preferences/model_configs.preferences.json");
        source.write(&path, &encrypted);
        source.write(&format!("{profile}/secrets/{}", SECRET_KEYS[0]), &key);
        originals.push((path, plaintext, encrypted));
    }
    let mut exporting = NativeRescueSession::open_file_secrets(source.0.join("runtime")).unwrap();
    exporting.execute(json!({"operation": "configure", "roots": source.roots()})).unwrap();
    let archive = exporting.execute(json!({"operation": "export"})).unwrap();
    let target = Fixture::new();
    let mut importing = NativeRescueSession::open_file_secrets(target.0.join("runtime")).unwrap();
    importing.execute(json!({"operation": "configure", "roots": target.roots()})).unwrap();
    let id = transfer(&archive, &mut importing);
    importing.execute(json!({"operation": "restore", "archiveId": id})).unwrap();
    for (path, plaintext, encrypted) in originals {
        assert_eq!(fs::read(source.0.join(&path)).unwrap(), encrypted);
        assert_eq!(fs::read(target.0.join(path)).unwrap(), plaintext);
    }
    for profile in ["runtime", "runtime/identities/a", "runtime/identities/b"] {
        let path = format!("{profile}/secrets/{}", SECRET_KEYS[0]);
        assert_eq!(fs::read(target.0.join(&path)).unwrap(), fs::read(source.0.join(path)).unwrap());
    }
}

/// Reports a missing profile-file key even when an obsolete secure-storage key still exists.
#[test]
fn profile_file_key_contract_rejects_missing_keys_without_reading_legacy_storage() {
    use operit_store::PreferencesEncryption::PreferencesEncryption;
    let source = Fixture::new();
    let key = serde_json::to_vec(&json!({
        "format": "operit.preferences.encryption.key", "version": 1,
        "algorithm": "XChaCha20Poly1305", "keyId": "obsolete-key",
        "key": base64::engine::general_purpose::STANDARD_NO_PAD.encode([7u8; 32]),
    })).unwrap();
    let storage_path = "runtime/config/preferences/model_configs.preferences.json";
    let encrypted = PreferencesEncryption::decodeKeyBytes(&key).unwrap().encrypt(storage_path, b"{}").unwrap();
    source.write("runtime/config/preferences/model_configs.preferences.json", &encrypted);
    source.write("secure/preferences_encryption_key.json", &key);
    let mut session = NativeRescueSession::open_file_secrets(source.0.join("runtime")).unwrap();
    session.execute(json!({"operation": "configure", "roots": source.roots()})).unwrap();
    assert_eq!(session.execute(json!({"operation": "export"})).unwrap_err(), format!(
        "{storage_path}: Cannot export preferences {storage_path}: Preferences encryption key is missing from the Host secret store"
    ));
    assert_eq!(fs::read(source.0.join("runtime/config/preferences/model_configs.preferences.json")).unwrap(), encrypted);
}

/// Keeps all normal processes protected while distinguishing maintenance conflicts from normal-startup conflicts.
#[test]
fn storage_leases_share_runtime_access_and_release_maintenance() {
    let fixture = Fixture::new();
    let runtime = fixture.0.join("runtime");
    let first = StorageLease::acquire(&runtime, false).unwrap();
    let second = StorageLease::acquire(&runtime, false).unwrap();
    let maintenance_error = StorageLease::acquire(&runtime, true).err().unwrap();
    drop(first);
    assert!(StorageLease::acquire(&runtime, true).is_err());
    drop(second);
    let maintenance = NativeRescueSession::open(runtime.clone(), None).unwrap();
    let runtime_error = StorageLease::acquire(&runtime, false).err().unwrap();
    assert_ne!(runtime_error, maintenance_error);
    assert!(!runtime_error.contains("Close all Operit windows"));
    drop(maintenance);
    let normal = StorageLease::acquire(&runtime, false).unwrap();
    drop(normal);
}
