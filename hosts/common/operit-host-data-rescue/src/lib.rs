//! Native Host boundary for offline rescue, process leases, and durable import publication.
#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use base64::{Engine, engine::general_purpose::STANDARD};
use fs2::FileExt;
use operit_data_rescue::{DataRescue, RescueFile, RescueHost, RescueManifest, payload_bytes, validate_path};
use operit_host_api::{HostSecretStore, RuntimeStorageHost};
use operit_host_native_storage::{ARCHIVE_STAGING_DIRECTORY_NAME, NativeArchiveStagingHost, NativeFileSecretStore, NativeRuntimeStorageHost};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use aes_gcm::{aead::{Aead, KeyInit, Payload}, Aes256Gcm, Nonce};

pub use operit_data_rescue::RescueProgress;

const BOOTSTRAP: &str = "runtime/link/local_runtime_storage.json";
const SECRET_KEYS: &[&str] = &["operit.preferences.encryption_key.v1", "market.github-oauth-pending"];
const SYSTEM_SECRETS: &str = "system/host-secrets.json";

/// Holds the operating-system file lock until every user of this lease has stopped.
pub struct StorageLease { _file: File }
impl StorageLease {
    /// Acquires shared runtime access or exclusive maintenance access without blocking startup.
    pub fn acquire(default_runtime: &Path, maintenance: bool) -> Result<Self, String> {
        let application = application_root(default_runtime)?;
        fs::create_dir_all(&application).map_err(text)?;
        let file = private_file(&application.join(".operit-data.lock"), false)?;
        let result = if maintenance { file.try_lock_exclusive() } else { FileExt::try_lock_shared(&file) };
        result.map_err(|error| match (maintenance, error.kind()) {
            (true, std::io::ErrorKind::WouldBlock) => format!("Data is in use. Close all Operit windows, background services, and CLI processes before data maintenance: {error}"),
            (false, std::io::ErrorKind::WouldBlock) => format!("Data maintenance is active. Close the Data Rescue session before normal startup: {error}"),
            _ => format!("Cannot acquire {} storage access at {}: {error}", if maintenance { "exclusive maintenance" } else { "shared runtime" }, application.display()),
        })?;
        if !maintenance && application.join("data-rescue/commit.json").exists() {
            return Err("An interrupted data import must be completed from Data Rescue before normal startup".into());
        }
        Ok(Self { _file: file })
    }
}

/// Selects complete base directories, including every isolated identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RescueRoots { pub runtime_root: PathBuf, pub workspace_root: PathBuf, #[serde(default)] pub cli_config_root: Option<PathBuf> }

/// Reads only the saved data-location contract without parsing identity or application state.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordedRescueRoots {
    confirmed: bool,
    runtime_root: PathBuf,
    workspace_root: PathBuf,
}

/// Owns a maintenance session independently of Flutter and Core.
pub struct NativeRescueSession {
    _lease: StorageLease,
    default_runtime: PathBuf,
    secret_store: Option<Arc<dyn HostSecretStore>>,
    profile_file_secrets: bool,
    service: Option<DataRescue>,
    progress: Arc<Mutex<Option<RescueProgress>>>,
}
impl NativeRescueSession {
    /// Opens exclusive maintenance access without parsing the startup configuration.
    pub fn open(default_runtime: PathBuf, secret_store: Option<Arc<dyn HostSecretStore>>) -> Result<Self, String> {
        let default_runtime = absolute_root(&default_runtime)?;
        let lease = StorageLease::acquire(&default_runtime, true)?;
        Ok(Self { _lease: lease, default_runtime, secret_store, profile_file_secrets: false, service: None, progress: Arc::new(Mutex::new(None)) })
    }

    /// Opens maintenance with the CLI profile-file secret contract for every identity.
    pub fn open_file_secrets(default_runtime: PathBuf) -> Result<Self, String> {
        let mut session = Self::open(default_runtime, None)?;
        session.profile_file_secrets = true;
        Ok(session)
    }

    /// Shares operation counters independently of the maintenance session lock.
    pub fn progress(&self) -> Arc<Mutex<Option<RescueProgress>>> { self.progress.clone() }

    /// Exposes raw startup bytes and platform defaults even when the saved configuration is damaged.
    pub fn describe_startup(&self) -> Result<Value, String> {
        let application = application_root(&self.default_runtime)?;
        let path = application.join("client/link/local_runtime_storage.json");
        let bootstrap = match fs::read(&path) {
            Ok(bytes) => Some(STANDARD.encode(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("Cannot read bootstrap configuration at {}: {error}", path.display())),
        };
        let commit_path = application.join("data-rescue/commit.json");
        let (recovery_roots, recovery_error, recovery_journal_base64) = match fs::read(&commit_path) {
            Ok(bytes) => match serde_json::from_slice::<Journal>(&bytes) {
                Ok(journal) if !journal.runtime_root.as_os_str().is_empty() && !journal.workspace_root.as_os_str().is_empty() => (
                    Some(json!({
                        "runtimeRoot": journal.runtime_root,
                        "workspaceRoot": journal.workspace_root,
                        "cliConfigRoot": journal.cli_config_root,
                    })), None, Some(STANDARD.encode(bytes)),
                ),
                Ok(_) => (None, Some("The pending import journal has no recorded recovery roots".to_string()), Some(STANDARD.encode(bytes))),
                Err(error) => (None, Some(format!("The pending import journal is invalid: {error}")), Some(STANDARD.encode(bytes))),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, None, None),
            Err(error) => return Err(format!("Cannot read pending import journal: {error}")),
        };
        Ok(json!({
            "defaultRoots": {"runtimeRoot": self.default_runtime, "workspaceRoot": application.join("workspaces")},
            "bootstrapBase64": bootstrap,
            "commitPending": commit_path.exists(),
            "recoveryRoots": recovery_roots,
            "recoveryError": recovery_error,
            "recoveryJournalBase64": recovery_journal_base64,
            "layout": "operit.files.v1"
        }))
    }

    /// Resolves the recorded locations without loading Core or requesting user-entered paths.
    fn automatic_roots(&self) -> Result<RescueRoots, String> {
        let startup = self.describe_startup()?;
        if startup["commitPending"] == true {
            if let Some(error) = startup["recoveryError"].as_str() {
                return Err(format!("无法定位待恢复的数据：{error}"));
            }
            return serde_json::from_value(startup["recoveryRoots"].clone())
                .map_err(|error| format!("中断的导入记录无法确定数据位置：{error}"));
        }
        let encoded = startup["bootstrapBase64"].as_str()
            .ok_or("找不到应用的数据位置记录，无法自动定位数据。")?;
        let bytes = STANDARD.decode(encoded).map_err(text)?;
        let config: RecordedRescueRoots = serde_json::from_slice(&bytes)
            .map_err(|error| format!("应用的数据位置记录损坏，无法自动定位数据：{error}"))?;
        if !config.confirmed {
            return Err("应用尚未完成数据位置设置，无法自动定位数据。".into());
        }
        if config.runtime_root.as_os_str().is_empty() || config.workspace_root.as_os_str().is_empty() {
            return Err("应用的数据位置记录为空，无法自动定位数据。".into());
        }
        Ok(RescueRoots {
            runtime_root: config.runtime_root,
            workspace_root: config.workspace_root,
            cli_config_root: None,
        })
    }

    /// Configures recorded or explicitly requested roots and executes an offline rescue command.
    pub fn execute(&mut self, request: Value) -> Result<Value, String> {
        if request["operation"] == "configureAutomatic" {
            let roots = self.automatic_roots()?;
            return self.execute(json!({"operation": "configure", "roots": roots}));
        }
        if request["operation"] == "configure" {
            if self.service.is_some() { return Err("Close the maintenance session before changing selected roots".into()); }
            let roots: RescueRoots = serde_json::from_value(request["roots"].clone()).map_err(text)?;
            let mut host = NativeRescueHost::new(&self.default_runtime, roots, self.secret_store.clone())?;
            host.profile_file_secrets = self.profile_file_secrets;
            host.progress = self.progress.clone();
            host.cleanup_abandoned_archives()?;
            let host = Arc::new(host);
            let archives = Arc::new(NativeArchiveStagingHost::new(application_root(&self.default_runtime)?.join("data-rescue/staging")));
            archives.cleanupAbandonedArchivesDuringMaintenance().map_err(text)?;
            self.service = Some(DataRescue { host, archives });
            return self.service.as_ref().ok_or("Rescue service was not created")?.host.describe();
        }
        if request["operation"] == "saveArchive" {
            let service = self.service.as_ref().ok_or("Prepare the data rescue storage session before continuing")?;
            let id = request["archiveId"].as_str().ok_or("Missing archive identifier")?;
            uuid::Uuid::parse_str(id).map_err(text)?;
            let destination = PathBuf::from(request["destination"].as_str().ok_or("Missing selected save destination")?);
            let source = PathBuf::from(service.archives.archiveFileReference(id).map_err(text)?);
            if archive_file_path(&source)? == archive_file_path(&destination)? { return Err("Save destination is the staged archive".into()); }
            let length = service.archives.sealArchive(id).map_err(text)?;
            let mut output = File::create(destination).map_err(text)?;
            let mut progress = RescueProgress::new("save", Some(length), "bytes");
            service.host.report_progress(progress.clone());
            while progress.completed < length {
                let count = (length - progress.completed).min(operit_data_rescue::CHUNK_BYTES as u64) as usize;
                let bytes = service.archives.readArchive(id, progress.completed, count).map_err(text)?;
                if bytes.len() != count { return Err("Truncated exported archive".into()); }
                output.write_all(&bytes).map_err(text)?;
                progress.advance(service.host.as_ref(), count as u64);
            }
            output.sync_all().map_err(text)?;
            return Ok(Value::Null);
        }
        self.service.as_ref().ok_or("Prepare the data rescue storage session before continuing")?.execute(request)
    }
}

/// Maps backup namespaces to Host-owned files while keeping every archive outside the data tree.
struct NativeRescueHost {
    application: PathBuf,
    roots: BTreeMap<String, PathBuf>,
    publication_roots: BTreeMap<String, PathBuf>,
    restore_writers: Mutex<BTreeMap<String, BTreeMap<String, bool>>>,
    secrets: Option<Arc<dyn HostSecretStore>>,
    profile_file_secrets: bool,
    progress: Arc<Mutex<Option<RescueProgress>>>,
}

/// Records a reversible filesystem move without automatically rolling back any operation.
#[derive(Clone, Serialize, Deserialize)]
struct Publication { target: PathBuf, staged: PathBuf, retained: PathBuf, had_original: bool, remove: bool, phase: String }

/// Blocks normal startup until every explicitly requested publication step has completed.
#[derive(Serialize, Deserialize)]
struct Journal {
    id: String,
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    cli_config_root: Option<PathBuf>,
    publications: Vec<Publication>,
    secret_envelope: Option<SecretEnvelope>,
    secret_index: usize,
    verified_files: Vec<RescueFile>,
    storage_nodes: Vec<StorageNode>,
}

/// Preserves empty directories and portable Unix permission bits in authenticated metadata.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StorageNode { path: String, directory: bool, unix_mode: Option<u32> }

/// Keeps imported and retained secrets encrypted under a platform secret-store key.
#[derive(Serialize, Deserialize)]
struct SecretEnvelope { nonce: String, ciphertext: String }

/// Retains original key material without leaving plaintext keys in publication catalogs.
#[derive(Serialize, Deserialize)]
struct JournalSecrets { imported: BTreeMap<String, String>, previous: BTreeMap<String, String> }

impl NativeRescueHost {
    /// Removes interrupted archive transfers from the exact Host-owned staging namespaces.
    fn cleanup_abandoned_archives(&self) -> Result<(), String> {
        for parent in [&self.application, &self.roots["runtime"], &self.roots["runtime"].join("identities")] {
            NativeArchiveStagingHost::cleanupAbandonedArchivesForRuntimeParent(parent).map_err(text)?;
        }
        Ok(())
    }

    /// Identifies temporary Host archives inside the selected complete runtime tree.
    fn is_archive_staging_path(&self, path: &Path) -> bool {
        path.starts_with(self.roots["runtime"].join(ARCHIVE_STAGING_DIRECTORY_NAME))
            || path.starts_with(self.roots["runtime"].join("identities").join(ARCHIVE_STAGING_DIRECTORY_NAME))
    }

    /// Enumerates complete filesystem structure without decoding any business records.
    fn inventory(&self) -> Result<(Vec<RescueFile>, Vec<StorageNode>), String> {
        let mut files = Vec::new();
        let mut nodes = Vec::new();
        for (role, root) in &self.roots {
            if root.exists() { collect_files(root, role, &mut files, &mut nodes, self)?; }
        }
        for entry in fs::read_dir(&self.application).map_err(text)? {
            let entry = entry.map_err(text)?;
            let name = entry.file_name().into_string().map_err(|_| "Application filename is not UTF-8")?;
            if !self.excluded_application_entry(&name) { collect_files(&entry.path(), &format!("application/{name}"), &mut files, &mut nodes, self)?; }
        }
        Ok((files, nodes))
    }

    /// Validates every filesystem destination and its exact authenticated node type.
    fn manifest_nodes(&self, manifest: &RescueManifest) -> Result<Vec<StorageNode>, String> {
        let nodes: Vec<StorageNode> = serde_json::from_value(manifest.metadata["storageNodes"].clone()).map_err(text)?;
        if nodes.len() > 1_000_000 { return Err("Archive exceeds the filesystem node limit".into()); }
        let mut paths = BTreeMap::new();
        let mut destinations = BTreeSet::new();
        let mut physical_nodes: BTreeMap<String, &StorageNode> = BTreeMap::new();
        let payloads: BTreeMap<_, _> = manifest.files.iter().map(|file| (file.path.as_str(), file)).collect();
        for node in &nodes {
            validate_path(&node.path)?;
            if !destinations.insert(portable_destination_key(&node.path)?) || paths.insert(node.path.as_str(), node.directory).is_some() {
                return Err(format!("Aliased or duplicate native destination: {}", node.path));
            }
            if let Some(root) = self.roots.get(&node.path) {
                if !node.directory { return Err("A storage root must be a directory".into()); }
                absolute_root(root)?;
            } else if node.path == "application" {
                return Err("The application namespace root is not archivable".into());
            } else { self.source_path(&node.path)?; }
            if node.unix_mode.is_some_and(|mode| mode & !0o777 != 0) { return Err("Unsupported permission bits in archive".into()); }
            let destination = self.source_path(&node.path)?.to_string_lossy().to_uppercase();
            if let Some(previous) = physical_nodes.insert(destination, node) {
                let same_payload = match (payloads.get(previous.path.as_str()), payloads.get(node.path.as_str())) {
                    (Some(left), Some(right)) => left.byte_length == right.byte_length && left.sha256 == right.sha256,
                    (None, None) => true,
                    _ => false,
                };
                if previous.directory != node.directory || previous.unix_mode != node.unix_mode || !same_payload {
                    return Err(format!("Conflicting archive entries share a destination: {} and {}", previous.path, node.path));
                }
            }
        }
        let files: BTreeSet<_> = manifest.files.iter().filter(|file| file.path != SYSTEM_SECRETS).map(|file| file.path.as_str()).collect();
        let declared: BTreeSet<_> = paths.iter().filter_map(|(path, directory)| (!directory).then_some(*path)).collect();
        if files != declared || manifest.files.iter().filter(|file| file.path == SYSTEM_SECRETS).count() != 1 {
            return Err("Native filesystem metadata does not match the authenticated payloads".into());
        }
        for node in &nodes {
            let destination = self.source_path(&node.path)?;
            for parent in destination.ancestors().skip(1) {
                if physical_nodes.get(&parent.to_string_lossy().to_uppercase()).is_some_and(|node| !node.directory) {
                    return Err(format!("Archive destination has a file as its parent: {}", node.path));
                }
            }
            for (index, _) in node.path.match_indices('/') {
                let ancestor = &node.path[..index];
                if ancestor == "application" { continue; }
                if paths.get(ancestor) != Some(&true) { return Err(format!("Missing directory metadata: {ancestor}")); }
            }
        }
        Ok(nodes)
    }

    /// Validates the selected roots before any data can be read or modified.
    fn new(default_runtime: &Path, selected: RescueRoots, secrets: Option<Arc<dyn HostSecretStore>>) -> Result<Self, String> {
        let application = application_root(default_runtime)?;
        let runtime = absolute_root(&selected.runtime_root)?;
        let workspaces = absolute_root(&selected.workspace_root)?;
        let client = application.join("client");
        let secure = runtime.parent().ok_or("Runtime root has no parent")?.join("secure");
        let mut roots: BTreeMap<String, PathBuf> = BTreeMap::from([
            ("runtime".into(), runtime), ("workspaces".into(), workspaces),
            ("client".into(), client), ("secure".into(), secure),
        ]);
        if let Some(root) = selected.cli_config_root { roots.insert("cli-config".into(), absolute_root(&root)?); }
        let mut candidates = BTreeMap::new();
        for (role, root) in &roots {
            if root.starts_with(application.join("data-rescue")) || application.starts_with(root) {
                return Err("Selected storage roots overlap maintenance storage".into());
            }
            let (role, target) = if root.starts_with(&application) {
                let relative = root.strip_prefix(&application).map_err(text)?;
                let name = relative.components().next().ok_or("Missing application storage entry")?;
                let target = application.join(name.as_os_str());
                let role = match roots.iter().find(|(_, root)| **root == target) {
                    Some((role, _)) => role.clone(),
                    None => format!("application-{}", name.as_os_str().to_str().ok_or("Storage directory is not UTF-8")?),
                };
                (role, target)
            } else { (role.clone(), root.clone()) };
            candidates.insert(role, target);
        }
        let mut publication_roots = BTreeMap::new();
        for (role, root) in &candidates {
            if candidates.iter().any(|(other_role, other)| root.starts_with(other) && (root != other || other_role < role)) { continue; }
            publication_roots.insert(role.clone(), root.clone());
        }
        Ok(Self { application, roots, publication_roots, restore_writers: Mutex::new(BTreeMap::new()), secrets, profile_file_secrets: false, progress: Arc::new(Mutex::new(None)) })
    }

    /// Returns the private maintenance catalog storage without entering any business service.
    fn catalog(&self) -> NativeRuntimeStorageHost {
        let root = self.application.join("data-rescue");
        NativeRuntimeStorageHost::new(root.clone(), root.join("unused-workspaces"))
    }

    /// Resolves one archive path only inside a declared source namespace.
    fn source_path(&self, path: &str) -> Result<PathBuf, String> {
        validate_path(path)?;
        if let Some(root) = self.roots.get(path) { return Ok(root.clone()); }
        let (role, relative) = path.split_once('/').ok_or("A rescue path needs a namespace")?;
        if role == "application" {
            let first = relative.split('/').next().ok_or("Missing application path")?;
            if self.excluded_application_entry(first) { return Err("Archive addresses a reserved maintenance path".into()); }
            return checked_descendant(&self.application, relative);
        }
        let root = self.roots.get(role).ok_or("Unknown rescue namespace")?;
        let path = checked_descendant(root, relative)?;
        if self.is_archive_staging_path(&path) { return Err("Archive addresses temporary archive staging".into()); }
        Ok(path)
    }

    /// Keeps generated executables, maintenance files, and declared roots out of the extra-data namespace.
    fn excluded_application_entry(&self, name: &str) -> bool {
        let path = self.application.join(name);
        [".operit-data.lock", "data-rescue", ARCHIVE_STAGING_DIRECTORY_NAME, "android-runtime"].contains(&name)
            || name.starts_with(".operit-rescue-") || name.starts_with(".operit-retained-")
            || self.roots.values().any(|root| root == &path)
    }

    /// Reads only registered persistent Host secrets into the portable backup payload.
    fn secret_payload(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&self.read_secrets()?).map_err(text)
    }

    /// Reads the portable secret values without exporting OS-specific wrapping keys.
    fn read_secrets(&self) -> Result<BTreeMap<String, String>, String> {
        let mut values = BTreeMap::new();
        if let Some(store) = &self.secrets {
            for key in SECRET_KEYS {
                if let Some(value) = store.readSecret(key).map_err(text)? { values.insert((*key).into(), STANDARD.encode(value)); }
            }
        }
        Ok(values)
    }

    /// Seals portable key material before any durable publication catalog is written.
    fn seal_journal_secrets(&self, id: &str, imported: BTreeMap<String, String>) -> Result<Option<SecretEnvelope>, String> {
        let Some(store) = &self.secrets else {
            if !imported.is_empty() { return Err("This Host cannot import system secrets".into()); }
            return Ok(None);
        };
        let mut key = [0u8; 32];
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut key).map_err(text)?;
        getrandom::fill(&mut nonce).map_err(text)?;
        let payload = serde_json::to_vec(&JournalSecrets { imported, previous: self.read_secrets()? }).map_err(text)?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(text)?;
        let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), Payload { msg: &payload, aad: id.as_bytes() }).map_err(text)?;
        store.writeSecret(&format!("operit.data-rescue.{id}.journal-key"), &key).map_err(text)?;
        Ok(Some(SecretEnvelope { nonce: STANDARD.encode(nonce), ciphertext: STANDARD.encode(ciphertext) }))
    }

    /// Authenticates retained key material through the platform's private secret store.
    fn open_journal_secrets(&self, journal: &Journal) -> Result<Option<JournalSecrets>, String> {
        let Some(envelope) = &journal.secret_envelope else { return Ok(None); };
        let store = self.secrets.as_ref().ok_or("The pending import requires the platform secret store")?;
        let key = store.readSecret(&format!("operit.data-rescue.{}.journal-key", journal.id)).map_err(text)?.ok_or("The pending import secret envelope key is unavailable")?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(text)?;
        let nonce = STANDARD.decode(&envelope.nonce).map_err(text)?;
        if nonce.len() != 12 { return Err("Invalid publication secret nonce".into()); }
        let ciphertext = STANDARD.decode(&envelope.ciphertext).map_err(text)?;
        let plaintext = cipher.decrypt(Nonce::from_slice(&nonce), Payload { msg: &ciphertext, aad: journal.id.as_bytes() }).map_err(text)?;
        let values: JournalSecrets = serde_json::from_slice(&plaintext).map_err(text)?;
        for (key, value) in &values.imported {
            if !SECRET_KEYS.contains(&key.as_str()) { return Err("Unregistered publication secret".into()); }
            STANDARD.decode(value).map_err(text)?;
        }
        Ok(Some(values))
    }

    /// Creates deterministic same-volume publication paths for one declared namespace.
    fn publication(&self, id: &str, role: &str, target: PathBuf) -> Result<Publication, String> {
        uuid::Uuid::parse_str(id).map_err(text)?;
        let parent = target.parent().ok_or("Import target has no parent")?;
        Ok(Publication {
            staged: parent.join(format!(".operit-rescue-{id}-{role}")),
            retained: parent.join(format!(".operit-retained-{id}-{role}")),
            had_original: target.exists(), remove: false, target, phase: "prepared".into(),
        })
    }

    /// Groups overlapping logical namespaces into one disjoint filesystem publication.
    fn publication_for_path(&self, id: &str, path: &Path) -> Result<Publication, String> {
        if let Some((role, root)) = self.publication_roots.iter().find(|(_, root)| path.starts_with(root)) {
            return self.publication(id, role, root.clone());
        }
        let relative = path.strip_prefix(&self.application).map_err(text)?;
        let name = relative.components().next().ok_or("Missing application publication entry")?.as_os_str();
        let name = name.to_str().ok_or("Application filename is not UTF-8")?;
        self.publication(id, &format!("application-{name}"), self.application.join(name))
    }

    /// Resolves every logical alias into its shared physical staging tree.
    fn staged_path(&self, id: &str, path: &str) -> Result<PathBuf, String> {
        uuid::Uuid::parse_str(id).map_err(text)?;
        validate_path(path)?;
        if path == SYSTEM_SECRETS { return Ok(self.application.join("data-rescue").join(id).join("host-secrets.json")); }
        let source = self.source_path(path)?;
        let plan = self.publication_for_path(id, &source)?;
        let relative = source.strip_prefix(&plan.target).map_err(text)?;
        if relative.as_os_str().is_empty() { return Ok(plan.staged); }
        checked_descendant(&plan.staged, relative.to_str().ok_or("Staged filename is not UTF-8")?.replace('\\', "/").as_str())
    }

    /// Allows exactly one authenticated archive entry to write each physical destination.
    fn writes_restore_file(&self, id: &str, path: &str) -> Result<bool, String> {
        self.restore_writers.lock().map_err(text)?.get(id)
            .ok_or("The restore has not been prepared")?.get(path).copied()
            .ok_or("The restore payload is not declared in its manifest".into())
    }

    /// Writes and flushes the journal atomically before any live directory is changed.
    fn save_journal(&self, journal: &Journal) -> Result<(), String> {
        self.catalog().writeBytesAtomically("runtime/commit.json", &serde_json::to_vec(journal).map_err(text)?).map_err(text)?;
        sync_directory(&self.application.join("data-rescue"))
    }

    /// Completes the persisted publication state machine without deleting previous data.
    fn publish(&self, mut journal: Journal) -> Result<Value, String> {
        self.verify_publication(&journal)?;
        let secrets = self.open_journal_secrets(&journal)?;
        let mut progress = RescueProgress::new("publish", Some((journal.publications.len() + if secrets.is_some() { SECRET_KEYS.len() } else { 0 } + 1) as u64), "items");
        self.report_progress(progress.clone());
        for index in 0..journal.publications.len() {
            let plan = journal.publications[index].clone();
            if plan.phase == "prepared" {
                if plan.had_original && !plan.retained.exists() {
                    durable_rename(&plan.target, &plan.retained)?;
                    sync_directory(plan.target.parent().ok_or("Publication target has no parent")?)?;
                }
                journal.publications[index].phase = "originalRetained".into();
                self.save_journal(&journal)?;
            }
            let plan = journal.publications[index].clone();
            if plan.phase == "originalRetained" {
                if plan.remove {
                    if plan.staged.exists() { return Err(format!("Unexpected staged data for removal: {}", plan.staged.display())); }
                } else {
                    if plan.staged.exists() {
                        durable_rename(&plan.staged, &plan.target)?;
                        sync_directory(plan.target.parent().ok_or("Publication target has no parent")?)?;
                    }
                    if !plan.target.exists() { return Err(format!("Import destination is missing: {}", plan.target.display())); }
                }
                journal.publications[index].phase = "published".into();
                self.save_journal(&journal)?;
            }
            progress.advance(self, 1);
        }
        if let Some(secrets) = secrets {
            let store = self.secrets.as_ref().ok_or("This Host cannot import system secrets")?;
            progress.advance(self, journal.secret_index as u64);
            for (index, key) in SECRET_KEYS.iter().enumerate().skip(journal.secret_index) {
                match secrets.imported.get(*key) {
                    Some(encoded) => store.writeSecret(key, &STANDARD.decode(encoded).map_err(text)?).map_err(text)?,
                    None => store.deleteSecret(key).map_err(text)?,
                }
                journal.secret_index = index + 1;
                self.save_journal(&journal)?;
                progress.advance(self, 1);
            }
        }
        let retained = self.application.join("data-rescue").join(&journal.id).join("completed.json");
        durable_rename(&self.application.join("data-rescue/commit.json"), &retained)?;
        sync_directory(&self.application.join("data-rescue"))?;
        sync_directory(retained.parent().ok_or("Retained catalog has no parent")?)?;
        progress.advance(self, 1);
        Ok(json!({"restored": true, "retainedData": journal.publications.iter().filter(|plan| plan.had_original).map(|plan| &plan.retained).collect::<Vec<_>>(), "retainedCatalog": retained}))
    }

    /// Rechecks every staged or already published payload before advancing a durable journal.
    fn verify_publication(&self, journal: &Journal) -> Result<(), String> {
        let mut files = journal.verified_files.clone();
        files.push(RescueFile { path: SYSTEM_SECRETS.into(), byte_length: 0, sha256: "0".repeat(64) });
        self.manifest_nodes(&RescueManifest {
            format: "operit.data-rescue".into(), version: 1, layout: self.layout().into(),
            files, metadata: json!({"storageNodes": journal.storage_nodes}),
        })?;
        let mut seen = BTreeSet::new();
        let mut node_paths = BTreeMap::new();
        for node in &journal.storage_nodes {
            validate_path(&node.path)?;
            let key = portable_destination_key(&node.path)?;
            if node_paths.insert(key, node).is_some() { return Err(format!("Duplicate publication node: {}", node.path)); }
            let live = match self.roots.get(&node.path) {
                Some(root) => root.clone(),
                None => self.source_path(&node.path)?,
            };
            let plan = journal.publications.iter().find(|plan| !plan.remove && live.starts_with(&plan.target)).ok_or("Publication node has no publication plan")?;
            let base = match (plan.phase.as_str(), plan.staged.exists(), plan.target.exists()) {
                ("prepared", true, _) | ("originalRetained", true, false) => &plan.staged,
                ("originalRetained", false, true) | ("published", false, true) => &plan.target,
                _ => return Err(format!("Publication node disagrees with recorded progress: {}", node.path)),
            };
            let relative = live.strip_prefix(&plan.target).map_err(text)?;
            let path = if relative.as_os_str().is_empty() { base.clone() } else { base.join(relative) };
            reject_linked_descendants(base, &path)?;
            let metadata = fs::symlink_metadata(&path).map_err(text)?;
            if is_storage_link(&metadata) || metadata.is_dir() != node.directory || (!node.directory && !metadata.is_file()) || node_mode(&metadata) != node.unix_mode {
                return Err(format!("Publication node metadata changed: {}", node.path));
            }
        }
        let mut progress = RescueProgress::new("verifyRestored", Some(payload_bytes(&journal.verified_files)?), "bytes");
        self.report_progress(progress.clone());
        for expected in &journal.verified_files {
            validate_path(&expected.path)?;
            if !seen.insert(&expected.path) { return Err("Duplicate publication integrity record".into()); }
            let live = self.source_path(&expected.path)?;
            let plan = journal.publications.iter().find(|plan| !plan.remove && live.starts_with(&plan.target)).ok_or("Publication integrity record has no destination")?;
            let base = match (plan.phase.as_str(), plan.staged.exists(), plan.target.exists()) {
                ("prepared", true, _) | ("originalRetained", true, false) => &plan.staged,
                ("originalRetained", false, true) | ("published", false, true) => &plan.target,
                _ => return Err(format!("Publication paths disagree with recorded progress: {}", live.display())),
            };
            let relative = live.strip_prefix(&plan.target).map_err(text)?;
            let path = base.join(relative);
            reject_linked_descendants(base, &path)?;
            let actual = hash_file(&path, &expected.path, expected.byte_length, self, &mut progress)?;
            if actual.byte_length != expected.byte_length || actual.sha256 != expected.sha256 { return Err(format!("Pending import payload integrity check failed: {}", expected.path)); }
        }
        Ok(())
    }
}

impl RescueHost for NativeRescueHost {
    /// Updates a short-lived counter lock independently of storage operations.
    fn report_progress(&self, progress: RescueProgress) {
        *self.progress.lock().expect("Rescue progress lock poisoned") = Some(progress);
    }

    /// Uses a portable directory layout for every native desktop and mobile Host.
    fn layout(&self) -> &str { "operit.files.v1" }

    /// Reports explicit data roots and the durable maintenance journal state.
    fn describe(&self) -> Result<Value, String> {
        let (_, nodes) = self.inventory()?;
        Ok(json!({"roots": self.roots, "applicationRoot": self.application, "commitPending": self.application.join("data-rescue/commit.json").exists(), "storageNodes": nodes}))
    }

    /// Enumerates all identities, workspaces, bootstrap data, legacy keys, and extra app files.
    fn list_files(&self) -> Result<Vec<RescueFile>, String> {
        if self.application.join("data-rescue/commit.json").exists() { return Err("Complete the interrupted import before exporting live data".into()); }
        let (mut files, _) = self.inventory()?;
        let payload = self.secret_payload()?;
        files.push(RescueFile { path: SYSTEM_SECRETS.into(), byte_length: payload.len() as u64, sha256: String::new() });
        Ok(files)
    }

    /// Reads regular files through a non-following, bounded storage boundary.
    fn read_file(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        if path == SYSTEM_SECRETS {
            let bytes = self.secret_payload()?;
            let start = usize::try_from(offset).map_err(text)?;
            let end = start.checked_add(length).ok_or("Secret range overflow")?.min(bytes.len());
            return bytes.get(start..end).map(<[u8]>::to_vec).ok_or("Invalid secret range".into());
        }
        let path = self.source_path(path)?;
        reject_symlink(&path)?;
        let mut input = File::open(path).map_err(text)?;
        input.seek(SeekFrom::Start(offset)).map_err(text)?;
        let mut bytes = vec![0; length];
        let count = input.read(&mut bytes).map_err(text)?;
        bytes.truncate(count);
        Ok(bytes)
    }

    /// Decrypts preference payloads using existing Host secrets without changing source files.
    fn export_preferences(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        use operit_store::PreferencesExport::{exportPreferences, isPreferencesPath, readPreferencesExportKey, MAX_PREFERENCES_EXPORT_BYTES};
        if path == SYSTEM_SECRETS { return Ok(None); }
        let source = self.source_path(path)?;
        let Ok(relative) = source.strip_prefix(&self.roots["runtime"]) else { return Ok(None); };
        let relative = relative.to_str().ok_or("Preferences path is not UTF-8")?.replace('\\', "/");
        let segments: Vec<_> = relative.split('/').collect();
        let (root, relative) = match segments.as_slice() {
            ["identities", identity, relative @ ..] => (
                self.roots["runtime"].join("identities").join(identity), relative.join("/"),
            ),
            _ => (self.roots["runtime"].clone(), relative),
        };
        let storage_path = format!("runtime/{relative}");
        if !isPreferencesPath(&storage_path) { return Ok(None); }
        reject_symlink(&source)?;
        let mut content = Vec::new();
        File::open(source).map_err(text)?.take(MAX_PREFERENCES_EXPORT_BYTES as u64 + 1).read_to_end(&mut content).map_err(text)?;
        if content.len() > MAX_PREFERENCES_EXPORT_BYTES { return Err(format!("Preferences exceed 16 MiB: {path}")); }
        let profile_secrets = NativeFileSecretStore(root.join("secrets"));
        let storage = NativeRuntimeStorageHost::new(root, self.roots["workspaces"].clone());
        let secrets = if self.profile_file_secrets {
            Some(&profile_secrets as &dyn HostSecretStore)
        } else {
            self.secrets.as_deref()
        };
        exportPreferences(&storage_path, content, || readPreferencesExportKey(&storage, secrets))
            .map(Some).map_err(|error| format!("{path}: {error}"))
    }

    /// Prepares empty same-volume directories and validates every manifest destination.
    fn begin_restore(&self, id: &str, manifest: &RescueManifest) -> Result<(), String> {
        uuid::Uuid::parse_str(id).map_err(text)?;
        if self.application.join("data-rescue/commit.json").exists() { return Err("Complete the interrupted import before starting another import".into()); }
        let nodes = self.manifest_nodes(manifest)?;
        fs::create_dir_all(self.application.join("data-rescue").join(id)).map_err(text)?;
        for file in &manifest.files { let _ = self.staged_path(id, &file.path)?; }
        for (role, root) in &self.publication_roots {
            let staged = self.publication(id, role, root.clone())?.staged;
            if staged.exists() { return Err("Import staging directory already exists".into()); }
            fs::create_dir_all(staged).map_err(text)?;
        }
        let mut progress = RescueProgress::new("prepareRestore", Some(nodes.iter().filter(|node| node.directory).count() as u64), "items");
        self.report_progress(progress.clone());
        for node in nodes.iter().filter(|node| node.directory) {
            fs::create_dir_all(self.staged_path(id, &node.path)?).map_err(text)?;
            progress.advance(self, 1);
        }
        let mut destinations = BTreeSet::new();
        let mut writers = BTreeMap::new();
        for file in &manifest.files {
            let destination = self.staged_path(id, &file.path)?.to_string_lossy().to_uppercase();
            writers.insert(file.path.clone(), destinations.insert(destination));
        }
        self.restore_writers.lock().map_err(text)?.insert(id.into(), writers);
        Ok(())
    }

    /// Appends bounded content only to this import's staging tree.
    fn write_restore(&self, id: &str, path: &str, chunk: &[u8]) -> Result<(), String> {
        if !self.writes_restore_file(id, path)? { return Ok(()); }
        let path = self.staged_path(id, path)?;
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(text)?; }
        private_file(&path, true)?.write_all(chunk).map_err(text)
    }

    /// Creates empty payloads and flushes staged bytes before publication can begin.
    fn finish_restore_file(&self, id: &str, path: &str) -> Result<(), String> {
        if !self.writes_restore_file(id, path)? { return Ok(()); }
        let path = self.staged_path(id, path)?;
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(text)?; }
        private_file(&path, true)?.sync_all().map_err(text)
    }

    /// Publishes verified directories and imported secrets under an explicit durable journal.
    fn commit_restore(&self, id: &str, manifest: &RescueManifest) -> Result<Value, String> {
        let result = (|| {
        let nodes = self.manifest_nodes(manifest)?;
        let secret_path = self.staged_path(id, SYSTEM_SECRETS)?;
        let secrets: BTreeMap<String, String> = serde_json::from_slice(&fs::read(secret_path).map_err(text)?).map_err(text)?;
        for (key, encoded) in &secrets {
            if !SECRET_KEYS.contains(&key.as_str()) { return Err(format!("Unregistered Host secret: {key}")); }
            STANDARD.decode(encoded).map_err(text)?;
        }
        if !secrets.is_empty() && self.secrets.is_none() { return Err("This Host does not support importing the archive's system secrets".into()); }
        let bootstrap = self.staged_path(id, "client/link/local_runtime_storage.json")?;
        if bootstrap.is_file() {
            let mut value: Value = serde_json::from_slice(&fs::read(&bootstrap).map_err(text)?).map_err(|error| format!("Imported startup configuration is invalid: {error}"))?;
            value["runtimeRoot"] = json!(self.roots["runtime"]);
            value["workspaceRoot"] = json!(self.roots["workspaces"]);
            let bytes = serde_json::to_vec(&value).map_err(text)?;
            let mut file = OpenOptions::new().write(true).truncate(true).open(bootstrap).map_err(text)?;
            file.write_all(&bytes).map_err(text)?;
            file.sync_all().map_err(text)?;
        }
        if self.roots.get("cli-config").is_some() {
            let config = self.staged_path(id, "cli-config/storage.json")?;
            if config.is_file() {
                let mut value: Value = serde_json::from_slice(&fs::read(&config).map_err(text)?).map_err(text)?;
                value["runtimeRoot"] = json!(self.roots["runtime"]);
                value["workspaceRoot"] = json!(self.roots["workspaces"]);
                let mut file = OpenOptions::new().write(true).truncate(true).open(config).map_err(text)?;
                file.write_all(&serde_json::to_vec(&value).map_err(text)?).map_err(text)?;
                file.sync_all().map_err(text)?;
            }
        }
        let mut publications = Vec::new();
        for (role, root) in &self.publication_roots { publications.push(self.publication(id, role, root.clone())?); }
        let extras: BTreeSet<String> = nodes.iter().filter_map(|node| node.path.strip_prefix("application/"))
            .filter_map(|path| path.split('/').next()).map(str::to_owned).collect();
        for name in &extras {
            let target = self.application.join(name);
            if !publications.iter().any(|plan| plan.target == target) {
                publications.push(self.publication(id, &format!("application-{name}"), target)?);
            }
        }
        for entry in fs::read_dir(&self.application).map_err(text)? {
            let entry = entry.map_err(text)?;
            let name = entry.file_name().into_string().map_err(|_| "Application filename is not UTF-8")?;
            if self.excluded_application_entry(&name) || extras.contains(&name) || publications.iter().any(|plan| plan.target == entry.path()) { continue; }
            reject_symlink(&entry.path())?;
            let mut plan = self.publication(id, &format!("application-remove-{name}"), entry.path())?;
            plan.remove = true;
            publications.push(plan);
        }
        let staged_bytes = manifest.files.iter().filter(|file| file.path != SYSTEM_SECRETS).try_fold(0u64, |total, file| {
            let length = fs::metadata(self.staged_path(id, &file.path)?).map_err(text)?.len();
            total.checked_add(length).ok_or_else(|| "Staged payload size overflow".to_string())
        })?;
        let mut progress = RescueProgress::new("verifyRestored", Some(staged_bytes), "bytes");
        self.report_progress(progress.clone());
        let mut verified_files = Vec::new();
        for file in manifest.files.iter().filter(|file| file.path != SYSTEM_SECRETS) {
            verified_files.push(hash_file(&self.staged_path(id, &file.path)?, &file.path, fs::metadata(self.staged_path(id, &file.path)?).map_err(text)?.len(), self, &mut progress)?);
        }
        for node in nodes.iter().filter(|node| !node.directory) { apply_node_mode(&self.staged_path(id, &node.path)?, node.unix_mode)?; }
        let directories: BTreeMap<_, _> = nodes.iter().filter(|node| node.directory)
            .map(|node| self.staged_path(id, &node.path).map(|path| (path, node.unix_mode)))
            .collect::<Result<_, _>>()?;
        let mut directories: Vec<_> = directories.into_iter().collect();
        directories.sort_by_key(|(path, _)| std::cmp::Reverse(path.components().count()));
        for (path, mode) in directories {
            sync_directory(&path)?;
            apply_node_mode(&path, mode)?;
        }
        let secret_envelope = self.seal_journal_secrets(id, secrets)?;
        fs::remove_file(self.staged_path(id, SYSTEM_SECRETS)?).map_err(text)?;
        let journal = Journal {
            id: id.into(),
            runtime_root: self.roots["runtime"].clone(),
            workspace_root: self.roots["workspaces"].clone(),
            cli_config_root: self.roots.get("cli-config").cloned(),
            publications,
            secret_envelope,
            secret_index: 0,
            verified_files,
            storage_nodes: nodes,
        };
        self.save_journal(&journal)?;
        self.restore_writers.lock().map_err(text)?.remove(id);
        self.publish(journal)
        })();
        if result.is_err() && !self.application.join("data-rescue/commit.json").exists() {
            if let Err(cleanup) = self.discard_restore(id) { return Err(format!("{}; staging cleanup failed: {cleanup}", result.unwrap_err())); }
        }
        result
    }

    /// Resumes only the publication plan explicitly persisted by a previous import.
    fn resume_restore(&self) -> Result<Value, String> {
        let journal: Journal = serde_json::from_slice(&self.catalog().readBytes("runtime/commit.json").map_err(text)?).map_err(text)?;
        uuid::Uuid::parse_str(&journal.id).map_err(text)?;
        if self.roots["runtime"] != journal.runtime_root || self.roots["workspaces"] != journal.workspace_root || self.roots.get("cli-config") != journal.cli_config_root.as_ref() {
            return Err("The selected roots do not match the pending import journal".into());
        }
        if journal.secret_index > SECRET_KEYS.len() { return Err("Invalid publication secret progress".into()); }
        let mut targets = BTreeSet::new();
        for plan in &journal.publications {
            absolute_root(&plan.target)?;
            if !targets.insert(&plan.target) || !["prepared", "originalRetained", "published"].contains(&plan.phase.as_str()) { return Err("Duplicate or invalid publication plan".into()); }
            let role = plan.staged.file_name().and_then(|name| name.to_str()).ok_or("Invalid staging name")?
                .strip_prefix(&format!(".operit-rescue-{}-", journal.id)).ok_or("Invalid journal staging path")?;
            if let Some(root) = self.publication_roots.get(role) {
                if plan.target != *root || plan.remove { return Err("Invalid declared-root publication".into()); }
            } else {
                let prefix = if plan.remove { "application-remove-" } else { "application-" };
                let name = role.strip_prefix(prefix).ok_or("Unknown publication namespace")?;
                validate_path(name)?;
                if name.split('/').count() != 1 || self.excluded_application_entry(name) || plan.target != self.application.join(name) {
                    return Err("Publication journal addresses a reserved destination".into());
                }
            }
            let expected = self.publication(&journal.id, role, plan.target.clone())?;
            if expected.staged != plan.staged || expected.retained != plan.retained { return Err("Invalid publication journal paths".into()); }
            reject_symlink(&plan.staged)?;
            reject_symlink(&plan.retained)?;
        }
        for root in self.publication_roots.values() {
            if !journal.publications.iter().any(|plan| !plan.remove && &plan.target == root) {
                return Err("The pending journal omits a selected storage root".into());
            }
        }
        let expected_extras: BTreeSet<_> = journal.storage_nodes.iter()
            .filter_map(|node| node.path.strip_prefix("application/"))
            .filter_map(|path| path.split('/').next())
            .filter(|name| !self.publication_roots.values().any(|root| root == &self.application.join(name))).collect();
        let published_extras: BTreeSet<_> = journal.publications.iter()
            .filter(|plan| !plan.remove && !self.publication_roots.values().any(|root| root == &plan.target))
            .map(|plan| plan.target.file_name().and_then(|name| name.to_str()).ok_or("Invalid application publication target"))
            .collect::<Result<_, _>>()?;
        if expected_extras != published_extras { return Err("The pending journal has inconsistent application destinations".into()); }
        self.publish(journal)
    }

    /// Deletes only uncommitted temporary directories derived from a validated UUID.
    fn discard_restore(&self, id: &str) -> Result<(), String> {
        uuid::Uuid::parse_str(id).map_err(text)?;
        if self.application.join("data-rescue/commit.json").exists() { return Err("Cannot discard a published import journal".into()); }
        let mut parents = vec![self.application.clone()];
        for root in self.publication_roots.values() {
            if let Some(parent) = root.parent() { parents.push(parent.to_path_buf()); }
        }
        parents.sort();
        parents.dedup();
        let prefix = format!(".operit-rescue-{id}-");
        for parent in parents {
            if !parent.is_dir() { continue; }
            for entry in fs::read_dir(&parent).map_err(text)? {
                let entry = entry.map_err(text)?;
                let name = entry.file_name().into_string().map_err(|_| "Staging filename is not UTF-8")?;
                if name.starts_with(&prefix) {
                    reject_symlink(&entry.path())?;
                    let metadata = fs::symlink_metadata(entry.path()).map_err(text)?;
                    if metadata.is_dir() { fs::remove_dir_all(entry.path()).map_err(text)?; }
                    else { fs::remove_file(entry.path()).map_err(text)?; }
                }
            }
        }
        let catalog = self.application.join("data-rescue").join(id);
        if catalog.exists() { fs::remove_dir_all(catalog).map_err(text)?; }
        if let Some(store) = &self.secrets { store.deleteSecret(&format!("operit.data-rescue.{id}.journal-key")).map_err(text)?; }
        self.restore_writers.lock().map_err(text)?.remove(id);
        Ok(())
    }
}

/// Enumerates raw regular files without opening SQLite, plugins, or application preferences.
fn collect_files(path: &Path, logical: &str, output: &mut Vec<RescueFile>, nodes: &mut Vec<StorageNode>, host: &NativeRescueHost) -> Result<(), String> {
    if host.is_archive_staging_path(path) { return Ok(()); }
    validate_path(logical)?;
    let metadata = fs::symlink_metadata(path).map_err(text)?;
    if is_storage_link(&metadata) { return Err(format!("Filesystem links require an explicit archival policy: {}", path.display())); }
    nodes.push(StorageNode { path: logical.into(), directory: metadata.is_dir(), unix_mode: node_mode(&metadata) });
    host.report_progress(RescueProgress { phase: "scan".into(), completed: nodes.len() as u64, total: None, unit: "items".into() });
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(text)? {
            let entry = entry.map_err(text)?;
            let name = entry.file_name().into_string().map_err(|_| "Data filename is not UTF-8")?;
            collect_files(&entry.path(), &format!("{logical}/{name}"), output, nodes, host)?;
        }
    } else if metadata.is_file() {
        output.push(RescueFile { path: logical.into(), byte_length: metadata.len(), sha256: String::new() });
    } else { return Err(format!("Unsupported storage object: {}", path.display())); }
    Ok(())
}

/// Rejects destination aliases that cannot be represented safely across native filesystems.
fn portable_destination_key(path: &str) -> Result<String, String> {
    for component in path.split('/') {
        if component.ends_with(['.', ' ']) || component.chars().any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '"' | '|' | '?' | '*')) {
            return Err(format!("Nonportable native destination: {path}"));
        }
        let stem = component.split('.').next().ok_or("Empty native path component")?.to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9") {
            return Err(format!("Reserved native destination: {path}"));
        }
    }
    Ok(path.to_uppercase())
}

/// Hashes a complete regular file without loading its contents into memory.
fn hash_file(path: &Path, logical: &str, expected_length: u64, host: &dyn RescueHost, progress: &mut RescueProgress) -> Result<RescueFile, String> {
    reject_symlink(path)?;
    let mut input = File::open(path).map_err(text)?;
    if !input.metadata().map_err(text)?.is_file() { return Err("Expected a regular publication payload".into()); }
    if input.metadata().map_err(text)?.len() != expected_length { return Err(format!("Publication size changed: {logical}")); }
    let mut hash = Sha256::new();
    let mut length = 0;
    let mut buffer = [0u8; 65536];
    while length < expected_length {
        let requested = (expected_length - length).min(buffer.len() as u64) as usize;
        let count = input.read(&mut buffer[..requested]).map_err(text)?;
        if count == 0 { return Err(format!("Publication payload was truncated: {logical}")); }
        hash.update(&buffer[..count]);
        length += count as u64;
        progress.advance(host, count as u64);
    }
    if input.read(&mut buffer[..1]).map_err(text)? != 0 { return Err(format!("Publication payload grew: {logical}")); }
    Ok(RescueFile { path: logical.into(), byte_length: length, sha256: format!("{:x}", hash.finalize()) })
}

/// Reads portable Unix permissions without preserving privileged execution bits.
#[cfg(unix)]
fn node_mode(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode() & 0o777)
}

/// Records that Windows ACLs belong to the destination application's security context.
#[cfg(windows)]
fn node_mode(_metadata: &fs::Metadata) -> Option<u32> { None }

/// Applies authenticated portable permission bits to a fully staged native entry.
#[cfg(unix)]
fn apply_node_mode(path: &Path, mode: Option<u32>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode { fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(text)?; }
    Ok(())
}

/// Keeps inherited private destination ACLs when importing portable native metadata.
#[cfg(windows)]
fn apply_node_mode(_path: &Path, _mode: Option<u32>) -> Result<(), String> { Ok(()) }

/// Resolves a normalized descendant and rejects existing symbolic-link components.
fn checked_descendant(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_path(relative)?;
    let mut result = root.to_path_buf();
    reject_symlink(&result)?;
    for part in relative.split('/') { result.push(part); reject_symlink(&result)?; }
    if !result.starts_with(root) { return Err("Rescue path escapes its root".into()); }
    Ok(result)
}

/// Rejects filesystem links without confusing a missing staging path with a read error.
fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if is_storage_link(&metadata) => Err(format!("Filesystem links are not accepted at the rescue boundary: {}", path.display())),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(text(error)),
    }
}

/// Detects symbolic links and Windows reparse points at the native rescue boundary.
fn is_storage_link(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() { return true; }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return metadata.file_attributes() & 0x400 != 0;
    }
    #[cfg(not(windows))]
    false
}

/// Resolves ancestor aliases into a physical data root while rejecting links at the root itself.
fn absolute_root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || path.components().any(|part| matches!(part, std::path::Component::ParentDir | std::path::Component::CurDir)) {
        return Err("Data roots must be normalized absolute directories".into());
    }
    // Only the selected root must not be redirected. Platform storage prefixes are routinely links
    // (Android: /data/user/0 <-> /data/data, macOS: /var -> /private/var), and every component
    // below the root is still checked by checked_descendant.
    reject_symlink(path)?;
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                if !fs::metadata(ancestor).map_err(text)?.is_dir() { return Err(format!("Data root is not a directory: {}", ancestor.display())); }
                let mut resolved = fs::canonicalize(ancestor).map_err(text)?;
                for name in missing.iter().rev() { resolved.push(name); }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(ancestor.file_name().ok_or("Data root has no directory name")?.to_os_string());
                ancestor = ancestor.parent().ok_or("Data root has no existing ancestor")?;
            }
            Err(error) => return Err(text(error)),
        }
    }
}

/// Resolves an archive file destination without applying directory-only root validation.
fn archive_file_path(path: &Path) -> Result<PathBuf, String> {
    reject_symlink(path)?;
    let parent = absolute_root(path.parent().ok_or("Archive file has no parent directory")?)?;
    let name = path.file_name().ok_or("Archive file has no filename")?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => fs::canonicalize(path).map_err(text),
        Ok(_) => Err("Archive destination must be a regular file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(parent.join(name)),
        Err(error) => Err(text(error)),
    }
}

/// Rejects links between a trusted publication base and one exact payload.
fn reject_linked_descendants(base: &Path, path: &Path) -> Result<(), String> {
    reject_symlink(base)?;
    let relative = path.strip_prefix(base).map_err(text)?;
    let mut current = base.to_path_buf();
    for part in relative.components() {
        current.push(part.as_os_str());
        reject_symlink(&current)?;
    }
    Ok(())
}

/// Resolves the fixed application-owned parent used for bootstrap and maintenance catalogs.
fn application_root(default_runtime: &Path) -> Result<PathBuf, String> {
    absolute_root(default_runtime)?.parent().map(Path::to_path_buf).ok_or("Default runtime root has no application parent".into())
}

/// Opens private backup metadata or staging files without widening existing permissions.
fn private_file(path: &Path, append: bool) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).append(append);
    set_private_creation_mode(&mut options);
    options.open(path).map_err(text)
}

/// Applies owner-only creation permissions at this native Host distribution boundary.
#[cfg(unix)]
fn set_private_creation_mode(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

/// Uses the application's inherited private ACL on Windows.
#[cfg(windows)]
fn set_private_creation_mode(_options: &mut OpenOptions) {}

/// Publishes a same-volume rename and synchronizes both affected directory entries.
#[cfg(unix)]
fn durable_rename(source: &Path, target: &Path) -> Result<(), String> {
    fs::rename(source, target).map_err(text)?;
    sync_directory(source.parent().ok_or("Rename source has no parent")?)?;
    sync_directory(target.parent().ok_or("Rename target has no parent")?)
}

/// Uses the Windows write-through move contract for native publication durability.
#[cfg(windows)]
fn durable_rename(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        return Err(text(std::io::Error::last_os_error()));
    }
    Ok(())
}

/// Flushes directory entries after journal and same-volume publication changes.
#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path).map_err(text)?.sync_all().map_err(text)
}

/// Relies on Windows native durable catalog replacement for directory publication metadata.
#[cfg(windows)]
fn sync_directory(_path: &Path) -> Result<(), String> { Ok(()) }

/// Preserves concrete Host errors in maintenance diagnostics.
fn text(error: impl std::fmt::Display) -> String { error.to_string() }

#[cfg(test)]
mod tests;
