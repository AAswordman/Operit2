use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use base64::{engine::general_purpose::STANDARD, Engine};
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore, PreferencesDataStore};
use crate::RuntimeFileSyncStore::RuntimeFileSyncStore;
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;

static CATALOG_REVISION: OnceLock<crate::PreferencesDataStore::MutableStateFlow<i64>> =
    OnceLock::new();

/// Observes completed local and remote extension catalog mutations.
pub fn catalogRevisionFlow() -> crate::PreferencesDataStore::StateFlow<i64> {
    CATALOG_REVISION
        .get_or_init(|| crate::PreferencesDataStore::MutableStateFlow::new(0))
        .asStateFlow()
}

/// Publishes one completed extension mutation to subscribed package-manager pages.
pub fn notifyCatalogChanged() {
    let flow =
        CATALOG_REVISION.get_or_init(|| crate::PreferencesDataStore::MutableStateFlow::new(0));
    flow.set_value(flow.value() + 1);
}

pub const SPACE_EXTENSION_RECORDS: &str = "runtime/extensions/space/records";

/// Installation identity and initial settings. File bytes are only retained for legacy migration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionRecord {
    pub kind: String,
    pub id: String,
    pub scope: String,
    pub sourceName: String,
    pub settings: Value,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
}

/// Stores independent extension entities through the cross-platform runtime storage host.
#[derive(Clone)]
pub struct ExtensionStore {
    storage: Arc<dyn RuntimeStorageHost>,
}

impl ExtensionStore {
    /// Opens the registered runtime storage host, without platform-specific storage branches.
    pub fn default() -> Self {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Opens an explicit host for isolated runtimes and tests.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }

    /// Copies pre-scope installations into the local scope once, preserving the original data
    /// and never overwriting installations already created with explicit scope selection.
    pub fn migrateLegacyLayout(&self) -> Result<(), String> {
        let marker = "runtime/extensions/device/legacy-layout-v1.complete";
        if self.storage.exists(marker).map_err(|e| e.to_string())? {
            return Ok(());
        }
        for directory in [
            "packages",
            "skills",
            "plugins/configs",
            "plugins/data",
            "mcp",
        ] {
            let old = format!("runtime/extensions/{directory}");
            let new = format!("runtime/extensions/device/{directory}");
            self.copyMissingTree(&old, &new)?;
        }
        self.copyMissingTree(
            operit_util::RuntimeStorageLayout::PACKAGE_MANAGER_PREFERENCES_PATH,
            "runtime/extensions/device/manager.preferences.json",
        )?;
        let mcp = "runtime/extensions/mcp/mcp_config.json";
        if self.storage.exists(mcp).map_err(|e| e.to_string())? {
            let config: Value =
                serde_json::from_slice(&self.storage.readBytes(mcp).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if let Some(servers) = config["mcpServers"].as_object() {
                for (id, server) in servers {
                    let metadata = config["pluginMetadata"].get(id).cloned().unwrap_or_else(||
                        serde_json::json!({"name": id, "description": "", "author": "Unknown", "version": "1.0.0"}));
                    self.registerDevice(
                        "mcp",
                        id,
                        "",
                        serde_json::json!({"server": server, "metadata": metadata}),
                    )?;
                }
            }
        }
        self.storage
            .writeBytes(marker, b"1")
            .map_err(|e| e.to_string())
    }

    fn copyMissingTree(&self, source: &str, target: &str) -> Result<(), String> {
        if !self.storage.exists(source).map_err(|e| e.to_string())? {
            return Ok(());
        }
        let parent = source.rsplit_once('/').ok_or("Invalid legacy path")?.0;
        let entry = self
            .storage
            .list(parent)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|entry| entry.path == source)
            .ok_or("Missing legacy entry")?;
        if entry.isDirectory {
            for child in self.storage.list(source).map_err(|e| e.to_string())? {
                let name = child
                    .path
                    .strip_prefix(&format!("{source}/"))
                    .ok_or("Legacy entry escaped directory")?;
                validateSegment(name)?;
                self.copyMissingTree(&child.path, &format!("{target}/{name}"))?;
            }
        } else if !self.storage.exists(target).map_err(|e| e.to_string())? {
            let bytes = self.storage.readBytes(source).map_err(|e| e.to_string())?;
            self.storage
                .writeBytes(target, &bytes)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn initialSettings(&self, kind: &str, id: &str, mut settings: Value) -> Result<Value, String> {
        use operit_util::RuntimeStorageLayout::{
            PACKAGE_MANAGER_PREFERENCES_PATH, SKILL_VISIBILITY_PREFERENCES_PATH,
        };
        let path = match kind {
            "package" => PACKAGE_MANAGER_PREFERENCES_PATH,
            "skill" => SKILL_VISIBILITY_PREFERENCES_PATH,
            _ => return Ok(settings),
        };
        if !self.storage.exists(path).map_err(|e| e.to_string())? {
            return Ok(settings);
        }
        let prefs = PreferencesDataStore::newWithStorage(self.storage.clone(), path)
            .data()
            .map_err(|e| e.to_string())?;
        if kind == "skill" {
            let hash = format!("{:x}", Sha256::digest(id.trim().as_bytes()));
            let legacy: String = id
                .trim()
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || matches!(c, '_' | '-') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            if let Some(value) = prefs
                .get(&stringPreferencesKey(&format!(
                    "skill_visible_{}",
                    &hash[..16]
                )))
                .or_else(|| prefs.get(&stringPreferencesKey(&format!("skill_visible_{legacy}"))))
            {
                settings["visible"] = serde_json::json!(value == "true");
            }
        } else {
            let members: Vec<String> =
                serde_json::from_value(settings["members"].clone()).map_err(|e| e.to_string())?;
            for (old, new) in [
                ("imported_packages", "enabledNames"),
                ("disabled_packages", "disabledNames"),
            ] {
                if let Some(value) = prefs.get(&stringPreferencesKey(old)) {
                    let values: Vec<String> =
                        serde_json::from_str(value).map_err(|e| e.to_string())?;
                    settings[new] = serde_json::json!(values
                        .into_iter()
                        .filter(|name| members.contains(name))
                        .collect::<Vec<_>>());
                }
            }
            if let Some(value) = prefs.get(&stringPreferencesKey("toolpkg_order")) {
                let order: Vec<String> = serde_json::from_str(value).map_err(|e| e.to_string())?;
                if let Some(index) = order.iter().position(|name| name == id) {
                    settings["order"] = serde_json::json!(index);
                }
            }
            if let Some(value) = prefs.get(&stringPreferencesKey("toolpkg_subpackage_states")) {
                let states: BTreeMap<String, bool> =
                    serde_json::from_str(value).map_err(|e| e.to_string())?;
                settings["subpackageStates"] = serde_json::json!(states
                    .into_iter()
                    .filter(|(name, _)| members.contains(name))
                    .collect::<BTreeMap<_, _>>());
            }
        }
        Ok(settings)
    }

    /// Resolves the single declared content directory for a validated scope and extension kind.
    pub fn root(kind: &str, scope: &str) -> Result<String, String> {
        let directory = match kind {
            "package" => "packages",
            "skill" => "skills",
            "mcp" => "mcp",
            _ => return Err(format!("Unknown extension kind: {kind}")),
        };
        validateScope(scope)?;
        Ok(format!("runtime/extensions/{scope}/{directory}"))
    }

    /// Lists every independently stored record, keeping conflicting identities visible to callers.
    pub fn records(&self, kind: &str) -> Result<Vec<ExtensionRecord>, String> {
        Self::root(kind, "device")?;
        let mut records = Vec::new();
        for scope in ["device", "space"] {
            for entry in self
                .storage
                .list(&format!("runtime/extensions/{scope}/records"))
                .map_err(|e| e.to_string())?
            {
                let name = entry
                    .path
                    .rsplit('/')
                    .next()
                    .ok_or("Invalid extension record path")?;
                if !name.starts_with(&format!("{kind}-")) {
                    continue;
                }
                if entry.isDirectory {
                    return Err("An extension record cannot be a directory".to_string());
                }
                let mut record: ExtensionRecord = serde_json::from_slice(
                    &self
                        .storage
                        .readBytes(&entry.path)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                self.validate(&record)?;
                if recordPath(&record.kind, &record.id, &record.scope)? != entry.path {
                    return Err("Extension identity does not match its record path".to_string());
                }
                if record.kind == kind {
                    // Old snapshots are upgraded in the owning repository, never in sync dispatch.
                    // Check actual files, not the received record, so interrupted upgrades can retry.
                    if !record.files.is_empty() {
                        self.upgradeLegacyBundle(&mut record)?;
                    }
                    let settings = self.settingsStore(&record)?;
                    if let Some(value) = settings
                        .data()
                        .map_err(|e| e.to_string())?
                        .get(&stringPreferencesKey("settings"))
                    {
                        record.settings = serde_json::from_str(value).map_err(|e| e.to_string())?;
                    }
                    self.validate(&record)?;
                    records.push(record);
                }
            }
        }
        Ok(records)
    }

    /// Resolves a package or subpackage owner from persisted membership without runtime locks.
    pub fn packageOwner(&self, packageName: &str) -> Result<ExtensionRecord, String> {
        let name = packageName.trim();
        let mut owner = None;
        for record in self.records("package")? {
            let members: Vec<String> = serde_json::from_value(record.settings["members"].clone())
                .map_err(|error| format!("Invalid package members: {error}"))?;
            if record.id == name || members.iter().any(|member| member == name) {
                if owner.is_some() {
                    return Err(format!("Extension ownership conflict: package:{name}"));
                }
                owner = Some(record);
            }
        }
        owner.ok_or_else(|| format!("Extension is not registered: package:{name}"))
    }

    /// Reads an exact identity and rejects multiple installed owners rather than overriding either.
    pub fn record(&self, kind: &str, id: &str) -> Result<ExtensionRecord, String> {
        let mut records = self
            .records(kind)?
            .into_iter()
            .filter(|record| record.id == id)
            .collect::<Vec<_>>();
        match records.len() {
            1 => Ok(records.remove(0)),
            0 => Err(format!("Extension is not registered: {kind}:{id}")),
            _ => Err(format!(
                "Extension ownership conflict: {kind}:{id} exists in both locations"
            )),
        }
    }

    /// Registers newly imported device content or a built-in resource with an explicit initial state.
    pub fn registerDevice(
        &self,
        kind: &str,
        id: &str,
        sourceName: &str,
        settings: Value,
    ) -> Result<(), String> {
        let records = self.records(kind)?;
        let matching = records
            .iter()
            .filter(|record| record.id == id)
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return Err(format!("Extension ownership conflict: {kind}:{id}"));
        }
        if matching.len() == 1 {
            return Ok(());
        }
        let record = ExtensionRecord {
            kind: kind.to_string(),
            id: id.to_string(),
            scope: "device".to_string(),
            sourceName: sourceName.to_string(),
            settings: self.initialSettings(kind, id, settings)?,
            files: BTreeMap::new(),
        };
        self.write(&record)
    }

    /// Retires an archived device package source without deleting its configuration or data.
    pub fn retireDevicePackageSource(&self, id: &str, sourceName: &str) -> Result<(), String> {
        let record = self.record("package", id)?;
        if record.scope != "device" || record.sourceName != sourceName {
            return Err(format!("Package retirement ownership mismatch: {id}/{sourceName}"));
        }
        let source = format!("{}/{}", Self::root("package", "device")?, sourceName);
        if self.storage.exists(&source).map_err(|error| error.to_string())? {
            return Err(format!("Package source must be archived before retirement: {sourceName}"));
        }
        self.storage.delete(&recordPath("package", id, "device")?, false)
            .map_err(|error| error.to_string())?;
        notifyCatalogChanged();
        Ok(())
    }

    /// Writes configuration in its owner scope and keeps portable file bytes unchanged.
    pub fn setSettings(&self, kind: &str, id: &str, settings: Value) -> Result<(), String> {
        let mut record = self.record(kind, id)?;
        if record.settings == settings {
            return Ok(());
        }
        record.settings = settings;
        self.validate(&record)?;
        self.writeSettings(&record)?;
        notifyCatalogChanged();
        Ok(())
    }

    /// Moves a complete extension and its configuration after validating target conflicts and deployment rules.
    pub fn moveScope(&self, kind: &str, id: &str, scope: &str) -> Result<(), String> {
        validateScope(scope)?;
        let journal = format!(
            "runtime/extensions/device/moves/{kind}-{:x}.json",
            Sha256::digest(id.as_bytes())
        );
        // Persist the source snapshot before touching the target. A failed move can resume even
        // after both records exist or only part of the source has been removed.
        let source = if self.storage.exists(&journal).map_err(|e| e.to_string())? {
            let source: ExtensionRecord = serde_json::from_slice(
                &self
                    .storage
                    .readBytes(&journal)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if source.kind != kind || source.id != id || source.scope == scope {
                return Err(
                    "An unfinished scope move must be retried in its original direction".into(),
                );
            }
            self.validate(&source)?;
            self.requireFileOnlyScopeMove(&source)?;
            source
        } else {
            let mut source = self.record(kind, id)?;
            if source.settings.get("builtin").and_then(Value::as_bool) == Some(true) {
                return Err(
                    "Built-in application resources do not have an installation scope".into(),
                );
            }
            if source.scope == scope {
                return Ok(());
            }
            let mut target = source.clone();
            target.scope = scope.to_string();
            self.validate(&target)?;
            for path in self
                .ownedPaths(&target)?
                .into_iter()
                .chain(std::iter::once(recordPath(kind, id, scope)?))
            {
                if self.storage.exists(&path).map_err(|e| e.to_string())? {
                    return Err(format!(
                        "Target location already contains extension content: {path}"
                    ));
                }
            }
            self.requireFileOnlyScopeMove(&source)?;
            source.files = self.snapshot(&source)?;
            self.storage
                .writeBytes(
                    &journal,
                    &serde_json::to_vec(&source).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            source
        };
        let mut target = source.clone();
        target.scope = scope.to_string();
        self.validate(&target)?;
        self.materialize(&target)?;
        target.files.clear();
        self.write(&target)?;
        self.remove(&source)?;
        self.storage
            .delete(&journal, false)
            .map_err(|e| e.to_string())?;
        notifyCatalogChanged();
        Ok(())
    }

    /// Deletes one exact installation and publishes a shared tombstone for a space-owned record.
    pub fn delete(&self, kind: &str, id: &str) -> Result<(), String> {
        let journal = format!(
            "runtime/extensions/device/deletions/{kind}-{:x}.json",
            Sha256::digest(id.as_bytes())
        );
        let record: ExtensionRecord = if self.storage.exists(&journal).map_err(|e| e.to_string())? {
            serde_json::from_slice(
                &self
                    .storage
                    .readBytes(&journal)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
        } else {
            let mut record = self.record(kind, id)?;
            record.files = self.snapshot(&record)?;
            self.storage
                .writeBytes(
                    &journal,
                    &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            record
        };
        if record.kind != kind || record.id != id {
            return Err("Deletion journal identity mismatch".into());
        }
        self.remove(&record)?;
        self.storage
            .delete(&journal, false)
            .map_err(|e| e.to_string())?;
        notifyCatalogChanged();
        Ok(())
    }

    /// Resolves a known package scope before registration without requiring an installation record.
    pub fn configPathForScope(id: &str, scope: &str) -> Result<String, String> {
        Self::root("package", scope)?;
        let name = operit_util::OperitPaths::pluginConfigDirName(id)?;
        Ok(format!("runtime/extensions/{scope}/plugins/configs/{name}"))
    }

    /// Resolves persistent plugin data from its explicit data scope rather than installation metadata.
    pub fn dataPathForScope(id: &str, scope: &str) -> Result<String, String> {
        validateScope(scope)?;
        let name = operit_util::OperitPaths::pluginConfigDirName(id)?;
        Ok(format!("runtime/plugin_data/{scope}/{name}"))
    }

    /// Resolves stable device-local plugin data independently of its installation scope.
    pub fn localDataPath(id: &str) -> Result<String, String> {
        Self::dataPathForScope(id, "device")
    }

    /// Resolves stable shared plugin data independently of its installation scope.
    pub fn spaceDataPath(id: &str) -> Result<String, String> {
        Self::dataPathForScope(id, "space")
    }

    /// Resolves the current scope-owned plugin config path using the existing stable directory naming rule.
    pub fn configPath(&self, id: &str) -> Result<String, String> {
        self.configRoot(&self.record("package", id)?)
    }

    /// Settings use the existing field-wise preference synchronization; installation bytes do not.
    fn settingsStore(&self, record: &ExtensionRecord) -> Result<PreferencesDataStore, String> {
        let path = recordPath(&record.kind, &record.id, &record.scope)?
            .replace("/records/", "/settings/")
            .replace(".json", ".preferences.json");
        Ok(if record.scope == "space" {
            PreferencesDataStore::newWithStorage(self.storage.clone(), path)
        } else {
            (*CoreNodeStateStore::newWithStorage(self.storage.clone(), path)).clone()
        }
        .withStructuredJsonSync())
    }

    fn writeSettings(&self, record: &ExtensionRecord) -> Result<(), String> {
        let value = serde_json::to_string(&record.settings).map_err(|e| e.to_string())?;
        self.settingsStore(record)?
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey("settings"), value);
            })
            .map_err(|e| e.to_string())
    }

    /// Compatibility with already installed bundles. Existing config is never overwritten;
    /// new peers receive ordinary file operations, not another embedded bundle.
    fn upgradeLegacyBundle(&self, record: &mut ExtensionRecord) -> Result<(), String> {
        for (relative, encoded) in &record.files {
            let path = self.filePath(record, relative)?;
            let bytes = if self.storage.exists(&path).map_err(|e| e.to_string())? {
                self.storage.readBytes(&path).map_err(|e| e.to_string())?
            } else {
                STANDARD.decode(encoded).map_err(|e| e.to_string())?
            };
            self.writeOwnedBytes(record, &path, &bytes)?;
        }
        // Commit the upgraded descriptor last. Failure leaves the bundle available for retry.
        record.files.clear();
        self.write(record)
    }

    /// Validates content paths and rejects space-local executable MCP definitions before any mutation.
    fn validate(&self, record: &ExtensionRecord) -> Result<(), String> {
        Self::root(&record.kind, &record.scope)?;
        if record.id.trim().is_empty() || !record.settings.is_object() {
            return Err("Extension requires a nonempty identity and object settings".to_string());
        }
        if !record.sourceName.is_empty() {
            validateSegment(&record.sourceName)?;
        }
        match record.kind.as_str() {
            "package" => {
                for field in ["members", "enabledNames", "disabledNames"] {
                    let _: Vec<String> = serde_json::from_value(record.settings[field].clone())
                        .map_err(|e| format!("Invalid package {field}: {e}"))?;
                }
                let _: BTreeMap<String, bool> =
                    serde_json::from_value(record.settings["subpackageStates"].clone())
                        .map_err(|e| e.to_string())?;
                if record.settings["order"].as_u64().is_none()
                    || record.settings["builtin"].as_bool().is_none()
                {
                    return Err("Invalid package ownership schema".to_string());
                }
                let identity = record.settings["installationId"]
                    .as_str()
                    .ok_or("Package installation identity is missing")?;
                if identity.len() != 32 || !identity.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("Invalid package installation identity".to_string());
                }
            }
            "skill" => {
                if record.settings["visible"].as_bool().is_none() {
                    return Err("Skill visibility must be a boolean".to_string());
                }
            }
            "mcp" => {
                if !record.settings["server"].is_object()
                    || !record.settings["metadata"].is_object()
                {
                    return Err("MCP definition and metadata must be objects".to_string());
                }
            }
            _ => return Err("Unknown extension kind".to_string()),
        }
        if record.kind == "mcp" && record.scope == "space" {
            let config = record
                .settings
                .get("server")
                .ok_or("MCP server definition is missing")?;
            let url = config
                .get("url")
                .and_then(Value::as_str)
                .ok_or("Local MCP can only belong to this device")?;
            if url.trim().is_empty()
                || config
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|command| !command.trim().is_empty())
            {
                return Err("Local MCP can only belong to this device".to_string());
            }
            if !record.sourceName.is_empty() {
                return Err(
                    "Shared remote MCP cannot contain a local deployment directory".to_string(),
                );
            }
        }
        for (relative, content) in &record.files {
            self.filePath(record, relative)?;
            STANDARD.decode(content).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Writes a validated complete record through the ownership-specific persistence API.
    fn write(&self, record: &ExtensionRecord) -> Result<(), String> {
        self.validate(record)?;
        let path = recordPath(&record.kind, &record.id, &record.scope)?;
        if !record.files.is_empty() {
            return Err("Installation records must not embed file content".into());
        }
        self.writeSettings(record)?;
        let content = serde_json::to_vec(record).map_err(|e| e.to_string())?;
        match record.scope.as_str() {
            "device" => self
                .storage
                .writeBytes(&path, &content)
                .map_err(|e| e.to_string()),
            "space" => RuntimeFileSyncStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
                .writeBytes(&path, &content),
            _ => Err("Unknown extension scope".to_string()),
        }
    }

    /// Removes owned content and records only after the target move has been published.
    fn remove(&self, record: &ExtensionRecord) -> Result<(), String> {
        self.deleteContent(record)?;
        self.settingsStore(record)?
            .edit(|preferences| {
                preferences.remove(&stringPreferencesKey("settings"));
            })
            .map_err(|e| e.to_string())?;
        let path = recordPath(&record.kind, &record.id, &record.scope)?;
        match record.scope.as_str() {
            "device" => self.storage.delete(&path, false).map_err(|e| e.to_string()),
            "space" => {
                RuntimeFileSyncStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH).delete(&path)
            }
            _ => Err("Unknown extension scope".to_string()),
        }
    }

    /// Deletes only the installation's validated content and configuration roots.
    fn deleteContent(&self, record: &ExtensionRecord) -> Result<(), String> {
        self.validate(record)?;
        // Include the move journal's original files, even if a previous attempt deleted them.
        let mut paths = std::collections::BTreeSet::new();
        for relative in record.files.keys() {
            paths.insert(self.filePath(record, relative)?);
        }
        for root in self.ownedPaths(record)? {
            self.collectFiles(&root, &mut paths)?;
        }
        let mut databaseFiles = std::collections::BTreeSet::new();
        for path in &paths {
            if crate::PluginStorage::ownsFile(self.storage.clone(), path).map_err(|e|e.to_string())? {
                databaseFiles.insert(path.clone());
            }
        }
        for path in &databaseFiles {
            if let Some(database) = path.strip_suffix(".operit-storage.json") {
                crate::PluginStorage::unregisterDatabase(crate::RuntimeStorageHost::defaultRuntimeSqliteHost(), database).map_err(|e|e.to_string())?;
            }
        }
        for path in paths {
            if databaseFiles.contains(&path) {
                if self.storage.exists(&path).map_err(|e| e.to_string())? { self.storage.delete(&path, false).map_err(|e|e.to_string())?; }
            } else if record.scope == "space" {
                RuntimeFileSyncStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
                    .delete(&path)?;
            } else if self.storage.exists(&path).map_err(|e| e.to_string())? {
                self.storage
                    .delete(&path, false)
                    .map_err(|e| e.to_string())?;
            }
        }
        for path in self.ownedPaths(record)? {
            if self.storage.exists(&path).map_err(|e| e.to_string())? {
                self.storage
                    .delete(&path, true)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn collectFiles(
        &self,
        path: &str,
        files: &mut std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        if !self.storage.exists(path).map_err(|e| e.to_string())? {
            return Ok(());
        }
        let parent = path.rsplit_once('/').ok_or("Invalid content path")?.0;
        let entry = self
            .storage
            .list(parent)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|entry| entry.path == path)
            .ok_or("Missing content entry")?;
        if entry.isDirectory {
            for child in self.storage.list(path).map_err(|e| e.to_string())? {
                self.collectFiles(&child.path, files)?;
            }
        } else {
            files.insert(path.to_string());
        }
        Ok(())
    }

    /// Returns the exact directories or archive files owned by one extension.
    fn ownedPaths(&self, record: &ExtensionRecord) -> Result<Vec<String>, String> {
        let mut paths = Vec::new();
        if !record.sourceName.is_empty() {
            paths.push(format!(
                "{}/{}",
                Self::root(&record.kind, &record.scope)?,
                record.sourceName
            ));
        }
        if record.kind == "package" {
            paths.push(self.configRoot(record)?);
            paths.push(self.configRoot(record)?.replacen("/plugins/configs/", "/plugins/data/", 1));
        }
        Ok(paths)
    }

    /// Resolves one package configuration root without changing its stable plugin directory name.
    fn configRoot(&self, record: &ExtensionRecord) -> Result<String, String> {
        Self::configPathForScope(&record.id, &record.scope)
    }

    /// Rejects scope relocation before mutation when a plugin owns transactional databases.
    fn requireFileOnlyScopeMove(&self, record: &ExtensionRecord) -> Result<(), String> {
        let mut paths = std::collections::BTreeSet::new();
        for root in self.ownedPaths(record)? { self.collectFiles(&root, &mut paths)?; }
        for path in paths {
            if crate::PluginStorage::ownsFile(self.storage.clone(), &path).map_err(|e|e.to_string())? { return Err("Plugin scope moves require an explicit database relocation transaction".into()); }
        }
        Ok(())
    }

    /// Captures the full portable content tree and existing configuration during a scope move.
    fn snapshot(&self, record: &ExtensionRecord) -> Result<BTreeMap<String, String>, String> {
        let mut files = BTreeMap::new();
        if !record.sourceName.is_empty() {
            let path = format!(
                "{}/{}",
                Self::root(&record.kind, &record.scope)?,
                record.sourceName
            );
            if !self.storage.exists(&path).map_err(|e| e.to_string())? {
                return Err(format!("Extension source is missing: {path}"));
            }
            if record.kind == "package" {
                files.insert(
                    format!("content/{}", record.sourceName),
                    STANDARD.encode(self.storage.readBytes(&path).map_err(|e| e.to_string())?),
                );
            } else {
                self.snapshotTree(&path, &format!("content/{}", record.sourceName), &mut files)?;
            }
        }
        if record.kind == "package" {
            self.snapshotTree(&self.configRoot(record)?, "config", &mut files)?;
            self.snapshotTree(&self.configRoot(record)?.replacen("/plugins/configs/", "/plugins/data/", 1), "data", &mut files)?;
        }
        Ok(files)
    }

    /// Recursively records host directory entries and rejects any listing outside its requested root.
    fn snapshotTree(
        &self,
        root: &str,
        prefix: &str,
        files: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        if !self.storage.exists(root).map_err(|e| e.to_string())? {
            return Ok(());
        }
        for entry in self.storage.list(root).map_err(|e| e.to_string())? {
            let name = entry
                .path
                .strip_prefix(&format!("{root}/"))
                .ok_or("Storage entry escaped its requested root")?;
            validateSegment(name)?;
            if crate::PluginStorage::ownsFile(self.storage.clone(), &entry.path).map_err(|e|e.to_string())? { continue; }
            let relative = format!("{prefix}/{name}");
            if entry.isDirectory {
                self.snapshotTree(&entry.path, &relative, files)?;
            } else {
                files.insert(
                    relative,
                    STANDARD.encode(
                        self.storage
                            .readBytes(&entry.path)
                            .map_err(|e| e.to_string())?,
                    ),
                );
            }
        }
        Ok(())
    }

    /// Maps a validated bundle entry to its strictly owned materialization path.
    fn filePath(&self, record: &ExtensionRecord, relative: &str) -> Result<String, String> {
        for segment in relative.split('/') {
            validateSegment(segment)?;
        }
        let (section, tail) = relative
            .split_once('/')
            .ok_or("Extension bundle section is missing")?;
        match section {
            "content" => {
                let allowed = match record.kind.as_str() {
                    "package" => tail == record.sourceName,
                    "skill" => tail
                        .strip_prefix(&format!("{}/", record.sourceName))
                        .is_some(),
                    _ => false,
                };
                if !allowed || record.sourceName.is_empty() {
                    return Err("Extension bundle escaped its owned source".to_string());
                }
                Ok(format!(
                    "{}/{tail}",
                    Self::root(&record.kind, &record.scope)?
                ))
            }
            "data" if record.kind == "package" => {
                Ok(format!("{}/{tail}", self.configRoot(record)?.replacen("/plugins/configs/", "/plugins/data/", 1)))
            }
            "config" if record.kind == "package" => {
                Ok(format!("{}/{tail}", self.configRoot(record)?))
            }
            _ => Err("Unknown extension bundle section".to_string()),
        }
    }

    /// Scope moves publish each file through the same path as any other synchronized file.
    fn writeOwnedBytes(
        &self,
        record: &ExtensionRecord,
        path: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        if record.scope == "space" {
            RuntimeFileSyncStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
                .writeBytes(path, bytes)
        } else {
            self.storage
                .writeBytes(path, bytes)
                .map_err(|e| e.to_string())
        }
    }

    fn materialize(&self, record: &ExtensionRecord) -> Result<(), String> {
        for (relative, encoded) in &record.files {
            self.writeOwnedBytes(
                record,
                &self.filePath(record, relative)?,
                &STANDARD.decode(encoded).map_err(|e| e.to_string())?,
            )?;
        }
        Ok(())
    }
}

/// Creates a collision-resistant entity path without deriving filesystem paths from display names.
fn recordPath(kind: &str, id: &str, scope: &str) -> Result<String, String> {
    ExtensionStore::root(kind, scope)?;
    if id.trim().is_empty() {
        return Err("Extension id cannot be empty".to_string());
    }
    Ok(format!(
        "runtime/extensions/{scope}/records/{kind}-{:x}.json",
        Sha256::digest(id.as_bytes())
    ))
}

/// Accepts only the two explicit installation scopes.
fn validateScope(scope: &str) -> Result<(), String> {
    match scope {
        "device" | "space" => Ok(()),
        _ => Err(format!("Unknown extension scope: {scope}")),
    }
}

/// Rejects absolute, traversing and multi-segment names at all filesystem boundaries.
fn validateSegment(segment: &str) -> Result<(), String> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '\0'))
    {
        return Err(format!("Invalid extension path segment: {segment}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/ExtensionStoreTests.rs"]
mod tests;
