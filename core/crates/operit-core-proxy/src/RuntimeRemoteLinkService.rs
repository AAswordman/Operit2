use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostRuntimeTaskSchedulerHost, TimeUtils::currentTimeMillis};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient, CoreValue};
use operit_link_access::{
    LinkAccessAutoSyncConfig, LinkAccessRoute, LinkAccessRoutingConfig, LinkAccessStore,
    PairedRemoteSession, PairedRemoteSessionRecord, PendingOutboundPairingRecord,
    RemoteDeviceInfo, RemoteLinkClient,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::RuntimeRemoteLinkDiscovery::discoverRemoteDevices;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot;

use crate::LocalCoreProxy;

const SYNC_DOMAINS: [&str; 3] = ["preferences", "chat", "objectbox"];
const AUTO_SYNC_INTERVAL_MS: u64 = 60_000;
static AUTO_SYNC_TASK_STARTED: OnceLock<Mutex<bool>> = OnceLock::new();

/// Reports the completed work from one runtime-owned paired remote sync transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemoteSyncResult {
    pub rounds: usize,
    pub localApplied: usize,
    pub remoteApplied: usize,
}

/// Reports the authenticated identity observed while probing one paired remote runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemoteProbeResult {
    pub coreDeviceId: String,
}

/// Reports the remote identity returned after beginning an outbound pairing transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemotePairStartResult {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
}

/// Describes a Link-enabled runtime discovered by the local runtime.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemoteDiscoveredDevice {
    pub deviceId: String,
    pub displayName: String,
    pub platform: String,
    pub model: String,
    pub baseUrl: String,
    pub hostname: String,
    pub port: u16,
    pub tokenHash: String,
    pub version: String,
}

/// Provides runtime-owned remote session operations to generated local Core clients.
#[derive(Clone)]
pub struct RuntimeRemoteLinkService {
    localCore: LocalCoreProxy,
    linkAccessStore: LinkAccessStore,
}

#[derive(Deserialize)]
struct SyncOperationOrder {
    opId: String,
    originDeviceId: String,
    sequence: i64,
    createdAt: i64,
}

impl RuntimeRemoteLinkService {
    /// Creates the service over the active local Core and its runtime-owned Link records.
    pub fn new(localCore: LocalCoreProxy) -> Self {
        let linkAccessStore = LinkAccessStore::new(localCore.runtimeStorageHost());
        Self {
            localCore,
            linkAccessStore,
        }
    }

    /// Selects the local Core as the destination for incoming Link requests.
    #[allow(non_snake_case)]
    pub fn selectLocalRoute(&self) -> Result<(), String> {
        self.linkAccessStore
            .saveRoutingConfig(LinkAccessRoutingConfig {
                route: LinkAccessRoute::Local,
                updatedAt: currentTimeMillis(),
            })
    }

    /// Selects one paired remote session as the destination for incoming Link requests.
    #[allow(non_snake_case)]
    pub fn selectPairedRemoteRoute(&self, name: String) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("paired remote session name must not be empty".to_string());
        }
        if !self.linkAccessStore.outboundSessions()?.contains_key(&name) {
            return Err(format!("paired remote runtime does not exist: {name}"));
        }
        self.linkAccessStore
            .saveRoutingConfig(LinkAccessRoutingConfig {
                route: LinkAccessRoute::Remote { sessionName: name },
                updatedAt: currentTimeMillis(),
            })
    }

    /// Returns the persisted runtime route, initializing it to the local runtime when absent.
    #[allow(non_snake_case)]
    pub fn currentRoute(&self) -> Result<LinkAccessRoutingConfig, String> {
        self.linkAccessStore.initializeRoutingConfig()
    }

    /// Returns every paired remote session owned by the local runtime.
    #[allow(non_snake_case)]
    pub fn pairedRemoteSessions(&self) -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
        self.linkAccessStore.outboundSessions()
    }

    /// Enables or disables automatic synchronization for one paired remote session.
    #[allow(non_snake_case)]
    pub fn setPairedRemoteAutoSync(
        &self,
        name: String,
        enabled: bool,
    ) -> Result<LinkAccessAutoSyncConfig, String> {
        if !self.linkAccessStore.outboundSessions()?.contains_key(&name) {
            return Err(format!("paired remote runtime does not exist: {name}"));
        }
        let mut config = self.linkAccessStore.initializeAutoSyncConfig()?;
        if enabled {
            if !config.autoSyncRemoteNames.contains(&name) {
                config.autoSyncRemoteNames.push(name);
            }
        } else {
            config.autoSyncRemoteNames.retain(|existing| existing != &name);
        }
        config.updatedAt = currentTimeMillis();
        self.linkAccessStore.saveAutoSyncConfig(config.clone())?;
        Ok(config)
    }

    /// Returns the current automatic synchronization configuration owned by this runtime.
    #[allow(non_snake_case)]
    pub fn autoSyncConfig(&self) -> Result<LinkAccessAutoSyncConfig, String> {
        self.linkAccessStore.initializeAutoSyncConfig()
    }

    /// Starts the singleton runtime-owned automatic synchronization worker.
    #[allow(non_snake_case)]
    pub fn startAutoSync(&self) -> Result<(), String> {
        let started = AUTO_SYNC_TASK_STARTED.get_or_init(|| Mutex::new(false));
        let mut started = started
            .lock()
            .map_err(|error| format!("automatic sync state lock poisoned: {error}"))?;
        if *started {
            return Ok(());
        }
        scheduleAutoSyncTick(self.clone(), 0)?;
        *started = true;
        Ok(())
    }

    /// Discovers Link-enabled runtimes through the native runtime transport.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    pub async fn discoverPairedRemotes(
        &self,
        timeoutMs: u64,
    ) -> Result<Vec<RuntimeRemoteDiscoveredDevice>, String> {
        if timeoutMs == 0 {
            return Err("remote discovery timeout must be greater than 0".to_string());
        }
        let (sender, receiver) = oneshot::channel();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeTask(
                "runtime-remote-discovery",
                Box::new(move || {
                    let _ = sender.send(discoverRemoteDevices(timeoutMs));
                }),
            )
            .map_err(|error| error.to_string())?;
        let devices = receiver
            .await
            .map_err(|_| "runtime discovery task ended before producing a result".to_string())??;
        self.refreshDiscoveredPairedRemoteEndpoints(&devices).await?;
        Ok(devices)
    }

    /// Runs automatic synchronization for every enabled paired remote endpoint.
    #[allow(non_snake_case)]
    pub async fn syncConfiguredPairedRemotes(&self) -> Result<(), String> {
        let config = self.linkAccessStore.initializeAutoSyncConfig()?;
        if config.autoSyncRemoteNames.is_empty() {
            return Ok(());
        }
        for name in config.autoSyncRemoteNames {
            self.syncPairedRemote(name, 512).await?;
        }
        Ok(())
    }

    /// Removes one paired remote while preserving a valid route and auto-sync configuration.
    #[allow(non_snake_case)]
    pub fn removePairedRemote(&self, name: String) -> Result<(), String> {
        if !self.linkAccessStore.outboundSessions()?.contains_key(&name) {
            return Err(format!("paired remote runtime does not exist: {name}"));
        }
        let route = self.linkAccessStore.initializeRoutingConfig()?;
        if route.route == (LinkAccessRoute::Remote {
            sessionName: name.clone(),
        }) {
            self.linkAccessStore
                .saveRoutingConfig(LinkAccessRoutingConfig {
                    route: LinkAccessRoute::Local,
                    updatedAt: currentTimeMillis(),
                })?;
        }
        let mut autoSync = self.linkAccessStore.initializeAutoSyncConfig()?;
        autoSync.autoSyncRemoteNames.retain(|existing| existing != &name);
        autoSync.updatedAt = currentTimeMillis();
        self.linkAccessStore.saveAutoSyncConfig(autoSync)?;
        self.linkAccessStore.removeOutboundSession(&name)
    }

    /// Starts a runtime-owned outbound pairing and stores its confidential client state.
    #[allow(non_snake_case)]
    pub async fn startPairedRemote(
        &self,
        baseUrl: String,
        tokenHash: String,
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<RuntimeRemotePairStartResult, String> {
        if baseUrl.trim().is_empty() {
            return Err("paired remote base URL must not be empty".to_string());
        }
        if tokenHash.trim().is_empty() {
            return Err("paired remote token hash must not be empty".to_string());
        }
        let client = RemoteLinkClient::new(baseUrl.clone());
        let hello = client.hello(&tokenHash).await?;
        let state = client.pairStart(&tokenHash, clientDeviceInfo).await?;
        if hello.coreDeviceId != state.coreDeviceId {
            return Err("paired remote identity changed during pairing".to_string());
        }
        self.linkAccessStore.savePendingOutboundPairing(
            state.pairingId.clone(),
            PendingOutboundPairingRecord { baseUrl, state: state.clone() },
        )?;
        Ok(RuntimeRemotePairStartResult {
            pairingId: state.pairingId,
            pairingServiceVersion: state.pairingServiceVersion,
            coreDeviceId: state.coreDeviceId,
            coreDeviceInfo: state.coreDeviceInfo,
        })
    }

    /// Completes a runtime-owned outbound pairing, stores its named session, and selects it.
    #[allow(non_snake_case)]
    pub async fn finishPairedRemote(
        &self,
        pairingId: String,
        pairingCode: String,
        name: String,
    ) -> Result<PairedRemoteSessionRecord, String> {
        if pairingId.trim().is_empty() {
            return Err("paired remote pairing id must not be empty".to_string());
        }
        if pairingCode.trim().is_empty() {
            return Err("paired remote pairing code must not be empty".to_string());
        }
        if name.trim().is_empty() {
            return Err("paired remote session name must not be empty".to_string());
        }
        if self.linkAccessStore.outboundSessions()?.contains_key(&name) {
            return Err(format!("paired remote session already exists: {name}"));
        }
        let pending = self
            .linkAccessStore
            .pendingOutboundPairings()?
            .get(&pairingId)
            .cloned()
            .ok_or_else(|| format!("pending paired remote does not exist: {pairingId}"))?;
        let client = RemoteLinkClient::new(pending.baseUrl);
        let record = client
            .pairFinish(&pending.state, &pairingCode)
            .await?
            .exportRecord();
        self.linkAccessStore
            .saveOutboundSession(name.clone(), record.clone())?;
        self.linkAccessStore
            .saveRoutingConfig(LinkAccessRoutingConfig {
                route: LinkAccessRoute::Remote { sessionName: name },
                updatedAt: currentTimeMillis(),
            })?;
        self.linkAccessStore.removePendingOutboundPairing(&pairingId)?;
        Ok(record)
    }

    /// Probes one named paired remote and verifies the persisted remote identity.
    #[allow(non_snake_case)]
    pub async fn probePairedRemote(
        &self,
        name: String,
    ) -> Result<RuntimeRemoteProbeResult, String> {
        let (record, session) = self.pairedSession(&name)?;
        let info = session.sessionInfo().await?;
        ensureRemoteIdentity(&record, &info.coreDeviceId)?;
        Ok(RuntimeRemoteProbeResult {
            coreDeviceId: info.coreDeviceId,
        })
    }

    /// Verifies and persists a discovered endpoint for one named paired remote runtime.
    #[allow(non_snake_case)]
    async fn updatePairedRemoteEndpoint(
        &self,
        name: String,
        baseUrl: String,
    ) -> Result<PairedRemoteSessionRecord, String> {
        let sessions = self.linkAccessStore.outboundSessions()?;
        let record = sessions
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("paired remote runtime does not exist: {name}"))?;
        let updated = record.withBaseUrl(baseUrl);
        let session = PairedRemoteSession::fromRecord(updated.clone())?;
        let info = session.sessionInfo().await?;
        ensureRemoteIdentity(&updated, &info.coreDeviceId)?;
        if updated.baseUrl != record.baseUrl {
            self.linkAccessStore
                .saveOutboundSession(name, updated.clone())?;
        }
        Ok(updated)
    }

    /// Runs one complete two-way sync transaction with a named paired remote runtime.
    #[allow(non_snake_case)]
    pub async fn syncPairedRemote(
        &self,
        name: String,
        limit: usize,
    ) -> Result<RuntimeRemoteSyncResult, String> {
        if limit == 0 {
            return Err("sync limit must be greater than 0".to_string());
        }
        let (record, session) = self.pairedSession(&name)?;
        let info = session.sessionInfo().await?;
        ensureRemoteIdentity(&record, &info.coreDeviceId)?;

        let localVersion = self.callLocal("coreVersion", Value::Null).await?;
        let remoteVersion = callRemote(&session, "coreVersion", Value::Null).await?;
        if localVersion != remoteVersion {
            return Err(format!(
                "core version mismatch: local={localVersion}, remote={remoteVersion}. sync blocked"
            ));
        }

        let mut rounds = 0;
        let mut localApplied = 0;
        let mut remoteApplied = 0;
        loop {
            rounds += 1;
            let localClock = self.callLocal("syncClock", Value::Null).await?;
            let remoteClock = callRemote(&session, "syncClock", Value::Null).await?;
            let localOperations = self
                .callLocal(
                    "syncOperationsSince",
                    json!({
                        "clock": remoteClock,
                        "domains": SYNC_DOMAINS,
                        "limit": limit,
                    }),
                )
                .await?;
            let remoteOperations = callRemote(
                &session,
                "syncOperationsSince",
                json!({
                    "clock": localClock,
                    "domains": SYNC_DOMAINS,
                    "limit": limit,
                }),
            )
            .await?;
            let operations = mergeSyncOperations(localOperations, remoteOperations)?;
            if operations.is_empty() {
                break;
            }
            let remoteResult = callRemote(
                &session,
                "syncApplyOperations",
                json!({ "operations": operations.clone() }),
            )
            .await?;
            let localResult = self
                .callLocal("syncApplyOperations", json!({ "operations": operations }))
                .await?;
            remoteApplied += appliedCount(remoteResult)?;
            localApplied += appliedCount(localResult)?;
            if operations.len() < limit {
                break;
            }
        }

        Ok(RuntimeRemoteSyncResult {
            rounds,
            localApplied,
            remoteApplied,
        })
    }

    /// Resolves a named persisted outbound record into its authenticated remote session.
    #[allow(non_snake_case)]
    fn pairedSession(
        &self,
        name: &str,
    ) -> Result<(PairedRemoteSessionRecord, PairedRemoteSession), String> {
        let sessions = self.linkAccessStore.outboundSessions()?;
        let record = sessions
            .get(name)
            .cloned()
            .ok_or_else(|| format!("paired remote runtime does not exist: {name}"))?;
        let session = PairedRemoteSession::fromRecord(record.clone())?;
        Ok((record, session))
    }

    /// Verifies and persists discovered endpoints for every matching paired remote session.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    async fn refreshDiscoveredPairedRemoteEndpoints(
        &self,
        devices: &[RuntimeRemoteDiscoveredDevice],
    ) -> Result<(), String> {
        let sessions = self.linkAccessStore.outboundSessions()?;
        for device in devices {
            for name in sessions
                .iter()
                .filter(|(_, session)| session.coreDeviceId == device.deviceId)
                .map(|(name, _)| name)
            {
                self.updatePairedRemoteEndpoint(name.clone(), device.baseUrl.clone())
                    .await?;
            }
        }
        Ok(())
    }

    /// Invokes one local application method through the active in-process Core.
    #[allow(non_snake_case)]
    async fn callLocal(&self, methodName: &str, args: Value) -> Result<Value, String> {
        let response =
            CoreLinkSharedClient::call(&self.localCore, applicationCallRequest(methodName, args)?)
                .await;
        coreResponseValue(response.result.map_err(|error| error.to_string())?)
    }
}

/// Schedules one Host-owned automatic sync tick and no persistent executor-owned timer.
#[allow(non_snake_case)]
fn scheduleAutoSyncTick(
    service: RuntimeRemoteLinkService,
    delayMs: u64,
) -> Result<(), String> {
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "runtime-remote-auto-sync-tick",
            delayMs,
            Box::new(move || {
                let syncService = service.clone();
                let scheduler = defaultHostRuntimeTaskSchedulerHost();
                if let Err(error) = scheduler.scheduleHostRuntimeAsyncTask(
                    "runtime-remote-auto-sync",
                    Box::new(move || {
                        Box::pin(async move {
                            if let Err(error) = syncService.syncConfiguredPairedRemotes().await {
                                operit_util::AppLogger::AppLogger::w(
                                    "RuntimeRemoteLinkService",
                                    &format!("automatic paired runtime sync failed: {error}"),
                                );
                            }
                        })
                    }),
                ) {
                    operit_util::AppLogger::AppLogger::w(
                        "RuntimeRemoteLinkService",
                        &format!("automatic paired runtime sync scheduling failed: {error}"),
                    );
                }
                if let Err(error) = scheduleAutoSyncTick(service, AUTO_SYNC_INTERVAL_MS) {
                    operit_util::AppLogger::AppLogger::w(
                        "RuntimeRemoteLinkService",
                        &format!("automatic paired runtime tick scheduling failed: {error}"),
                    );
                }
            }),
        )
        .map_err(|error| error.to_string())
}

/// Invokes one application method through an authenticated paired remote session.
#[allow(non_snake_case)]
async fn callRemote(
    session: &PairedRemoteSession,
    methodName: &str,
    args: Value,
) -> Result<Value, String> {
    let response = session
        .call(applicationCallRequest(methodName, args)?)
        .await?;
    coreResponseValue(response.result.map_err(|error| error.to_string())?)
}

/// Builds one Link request for an application-level runtime operation.
#[allow(non_snake_case)]
fn applicationCallRequest(methodName: &str, args: Value) -> Result<CoreCallRequest, String> {
    Ok(CoreCallRequest::new(
        format!("runtime-remote-{methodName}-{}", currentTimeMillis()),
        "application",
        methodName,
        toCoreValue(args).map_err(|error| error.to_string())?,
    ))
}

/// Decodes one successful Link response value into structured JSON.
#[allow(non_snake_case)]
fn coreResponseValue(value: CoreValue) -> Result<Value, String> {
    fromCoreValue(value).map_err(|error| error.to_string())
}

/// Verifies that the endpoint answered for the paired runtime identity stored locally.
#[allow(non_snake_case)]
fn ensureRemoteIdentity(
    record: &PairedRemoteSessionRecord,
    coreDeviceId: &str,
) -> Result<(), String> {
    if coreDeviceId != record.coreDeviceId {
        return Err("remote runtime identity changed".to_string());
    }
    Ok(())
}

/// Merges two operation pages into their deterministic application order.
#[allow(non_snake_case)]
fn mergeSyncOperations(left: Value, right: Value) -> Result<Vec<Value>, String> {
    let mut byId = BTreeMap::new();
    for operation in syncOperations(left)?
        .into_iter()
        .chain(syncOperations(right)?)
    {
        let key: SyncOperationOrder = serde_json::from_value(operation.clone())
            .map_err(|error| format!("invalid sync operation: {error}"))?;
        byId.insert(key.opId.clone(), (key, operation));
    }
    let mut operations = byId.into_values().collect::<Vec<_>>();
    operations.sort_by(|left, right| {
        (
            left.0.createdAt,
            &left.0.originDeviceId,
            left.0.sequence,
            &left.0.opId,
        )
            .cmp(&(
                right.0.createdAt,
                &right.0.originDeviceId,
                right.0.sequence,
                &right.0.opId,
            ))
    });
    Ok(operations
        .into_iter()
        .map(|(_, operation)| operation)
        .collect())
}

/// Decodes one runtime sync operation page into its ordered operation array.
#[allow(non_snake_case)]
fn syncOperations(value: Value) -> Result<Vec<Value>, String> {
    serde_json::from_value(value).map_err(|error| format!("invalid sync operations: {error}"))
}

/// Reads the applied operation count from one runtime sync apply response.
#[allow(non_snake_case)]
fn appliedCount(value: Value) -> Result<usize, String> {
    let applied = value
        .get("applied")
        .and_then(Value::as_u64)
        .ok_or_else(|| "sync apply response is missing an applied count".to_string())?;
    usize::try_from(applied).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies duplicate operations are collapsed and the merged page has stable sync order.
    #[test]
    fn merge_sync_operations_deduplicates_and_orders_operations() {
        let operations = mergeSyncOperations(
            json!([
                {
                    "opId": "device-a:2",
                    "originDeviceId": "device-a",
                    "sequence": 2,
                    "createdAt": 20,
                },
                {
                    "opId": "device-a:1",
                    "originDeviceId": "device-a",
                    "sequence": 1,
                    "createdAt": 10,
                },
            ]),
            json!([
                {
                    "opId": "device-b:1",
                    "originDeviceId": "device-b",
                    "sequence": 1,
                    "createdAt": 10,
                },
                {
                    "opId": "device-a:2",
                    "originDeviceId": "device-a",
                    "sequence": 2,
                    "createdAt": 20,
                },
            ]),
        )
        .expect("sync operations must merge");

        let ids = operations
            .iter()
            .map(|operation| operation["opId"].as_str().expect("operation id"))
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["device-a:1", "device-b:1", "device-a:2"]);
    }
}
