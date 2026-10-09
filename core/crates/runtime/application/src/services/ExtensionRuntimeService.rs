//! Extension activation is a consumer of committed data, not part of sync dispatch.
use operit_host_api::HostManager::HostManager;
use operit_store::PreferencesDataStore::PreferencesSyncedEntry;
use operit_store::SyncAppliedChanges::{self, SyncAppliedSubscription};
use operit_store::SyncOperationStore::SyncOperation;
use operit_tools::tools::mcp::MCPManager::MCPManager;
use operit_tools::tools::mcp_runtime::plugins::{MCPBridge::MCPBridge, MCPStarter::MCPStarter};
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::AppLogger::AppLogger;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default)]
struct Changes {
    packages: bool,
    settings: bool,
    mcp: bool,
    catalog: bool,
}
impl Changes {
    fn merge(&mut self, other: Self) {
        self.packages |= other.packages;
        self.settings |= other.settings;
        self.mcp |= other.mcp;
        self.catalog |= other.catalog;
    }
}
#[derive(Default)]
struct Pending {
    changes: Changes,
    scheduled: bool,
}
struct Worker {
    host: HostManager,
    tools: AIToolHandler,
    pending: Mutex<Pending>,
}

/// The owning application keeps this subscription alive and drops it on shutdown.
pub fn start(host: HostManager, tools: AIToolHandler) -> Option<SyncAppliedSubscription> {
    let storage = host.runtimeStorageHost.clone()?;
    let worker = Arc::new(Worker {
        host,
        tools,
        pending: Mutex::new(Pending::default()),
    });
    Some(SyncAppliedChanges::subscribe(storage, move |operations| {
        let changes = classify(operations);
        if !changes.catalog {
            return;
        }
        let mut pending = worker.pending.lock().expect("extension refresh poisoned");
        pending.changes.merge(changes);
        if pending.scheduled {
            return;
        }
        pending.scheduled = true;
        drop(pending);
        let task = worker.clone();
        let result = worker
            .host
            .hostRuntimeTaskSchedulerHost
            .as_ref()
            .ok_or_else(|| "Runtime scheduler is required for extension activation".to_string())
            .and_then(|scheduler| {
                scheduler
                    .scheduleHostRuntimeAsyncTask(
                        "operit-extension-refresh",
                        Box::new(move || Box::pin(async move { task.run().await })),
                    )
                    .map_err(|e| e.to_string())
            });
        if let Err(error) = result {
            worker
                .pending
                .lock()
                .expect("extension refresh poisoned")
                .scheduled = false;
            AppLogger::e("ExtensionRuntimeService", &error);
        }
    }))
}

fn classify(operations: &[SyncOperation]) -> Changes {
    let mut changes = Changes::default();
    for operation in operations {
        let path = match operation.domain.as_str() {
            "runtime_file" => operation.entityId.clone(),
            "preferences" => match PreferencesSyncedEntry::fromOperation(operation) {
                Ok(entry) => entry.storagePath().to_string(),
                Err(_) => continue,
            },
            _ => continue,
        };
        let Some(relative) = path.strip_prefix("runtime/extensions/space/") else {
            continue;
        };
        changes.catalog = true;
        changes.packages |=
            relative.starts_with("packages/") || relative.starts_with("records/package-");
        changes.settings |= relative.starts_with("settings/package-");
        changes.mcp |=
            relative.starts_with("records/mcp-") || relative.starts_with("settings/mcp-");
    }
    changes
}

impl Worker {
    async fn run(&self) {
        // Retry activation independently of already committed data. Failed work remains pending
        // for the next notification; normal startup also reconstructs runtime state from storage.
        let mut failures = 0;
        loop {
            let changes = {
                let mut pending = self.pending.lock().expect("extension refresh poisoned");
                if !pending.changes.catalog {
                    pending.scheduled = false;
                    return;
                }
                std::mem::take(&mut pending.changes)
            };
            match self.refresh(changes).await {
                Ok(()) => failures = 0,
                Err(error) => {
                    AppLogger::e(
                        "ExtensionRuntimeService",
                        &format!("Activation failed; sync data retained: {error}"),
                    );
                    let mut pending = self.pending.lock().expect("extension refresh poisoned");
                    pending.changes.merge(changes);
                    failures += 1;
                    if failures >= 3 {
                        pending.scheduled = false;
                        return;
                    }
                }
            }
        }
    }

    async fn refresh(&self, changes: Changes) -> Result<(), String> {
        if changes.packages || changes.settings {
            let storage = self
                .host
                .runtimeStorageHost
                .clone()
                .ok_or("Runtime storage is missing")?;
            // Compatibility upgrades must finish before scanning the installation directory.
            operit_store::ExtensionStore::ExtensionStore::new(storage).records("package")?;
            let manager = self.tools.getOrCreatePackageManager();
            let mut manager = manager.lock().map_err(|e| e.to_string())?;
            if changes.packages {
                manager.loadAvailablePackages();
            } else {
                manager.refreshExtensionSettings()?;
            }
        }
        if changes.mcp {
            // Propagate malformed/conflicting definitions BEFORE making destructive changes.
            let local = MCPLocalServer::getInstance(&self.host);
            let definitions = local.getAllMCPServersChecked()?;
            let manager = MCPManager::getInstance(self.host.clone());
            let bridge = MCPBridge::getInstance(&self.host);
            for id in manager.getRegisteredServers().keys() {
                if definitions.get(id).is_none_or(|server| server.disabled) {
                    manager.unregisterServer(id);
                    bridge.unregisterMcpService(id);
                }
            }
            let storage = self
                .host
                .runtimeStorageHost
                .clone()
                .ok_or("Runtime storage is missing")?;
            let records =
                operit_store::ExtensionStore::ExtensionStore::new(storage).records("mcp")?;
            let starter = MCPStarter::new(self.host.clone());
            for record in records.into_iter().filter(|record| record.scope == "space") {
                if definitions
                    .get(&record.id)
                    .is_some_and(|server| !server.disabled)
                    && !starter.startPlugin(&record.id, |_| {}).await
                {
                    return Err(format!("Unable to activate shared MCP: {}", record.id));
                }
            }
        }
        operit_store::ExtensionStore::notifyCatalogChanged();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_store::SyncOperationStore::SyncOperationSemantics;
    use serde_json::{json, Value};

    fn operation(domain: &str, path: &str) -> SyncOperation {
        let (entityId, payload) = if domain == "preferences" {
            (
                serde_json::to_string(&(path, "settings")).unwrap(),
                json!({
                    "storagePath": path, "key": "settings", "value": "{}", "encrypted": false
                }),
            )
        } else {
            (path.to_string(), Value::Null)
        };
        SyncOperation {
            opId: "test:1".into(),
            originDeviceId: "test".into(),
            sequence: 1,
            domain: domain.into(),
            entityType: "file".into(),
            entityId,
            operation: if domain == "preferences" {
                "set"
            } else {
                "upsert"
            }
            .into(),
            semantics: SyncOperationSemantics::EntityState,
            payload,
            createdAt: 0,
            schemaVersion: 1,
        }
    }

    #[test]
    fn config_and_skill_changes_do_not_reload_plugins_or_restart_mcp() {
        for path in [
            "plugins/configs/demo/env.json",
            "skills/demo/SKILL.md",
            "records/skill-demo.json",
        ] {
            let changes = classify(&[operation(
                "runtime_file",
                &format!("runtime/extensions/space/{path}"),
            )]);
            assert!(changes.catalog);
            assert!(!changes.packages && !changes.settings && !changes.mcp);
        }
    }

    #[test]
    fn package_flags_refresh_state_without_destroying_engines() {
        let changes = classify(&[operation(
            "preferences",
            "runtime/extensions/space/settings/package-demo.preferences.json",
        )]);
        assert!(changes.catalog && changes.settings);
        assert!(!changes.packages && !changes.mcp);
    }

    #[test]
    fn mcp_changes_do_not_reload_packages_and_unrelated_files_are_ignored() {
        let changes = classify(&[operation(
            "preferences",
            "runtime/extensions/space/settings/mcp-demo.preferences.json",
        )]);
        assert!(changes.catalog && changes.mcp);
        assert!(!changes.packages && !changes.settings);
        assert!(!classify(&[operation("runtime_file", "runtime/data/unrelated.json")]).catalog);
    }
}
