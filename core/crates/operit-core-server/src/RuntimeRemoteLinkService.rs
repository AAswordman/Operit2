#[cfg(not(target_arch = "wasm32"))]
use crate::RuntimeRemoteLinkDiscovery::{discoverRemoteDevices, RuntimeRemoteDiscoveryEndpoint};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostRuntimeTaskSchedulerHost, TimeUtils::currentTimeMillis};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreValue};
use operit_link_access::{
    AcceptedRemoteSessionRecord,
    CoreNodePeerLink::{
        activePeerNodeIds, disconnectPeerLink, isPeerLinkActive, kickPeerLink,
    },
    LinkAccessStore, PairedRemoteSession, PairedRemoteSessionRecord, PendingOutboundPairingRecord,
    LinkTransportPreference, RemoteDeviceInfo, RemoteLinkClient,
};
use operit_store::CoreNodeBindingStore::CoreNodeBindingStore;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile, CoreSpaceStore};
use operit_store::PreferencesDataStore::{combine2, CoroutineScope, SharingStarted, StateFlow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot;

use crate::{
    CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter},
    SpacePersistenceSyncService::SpacePersistenceSyncService,
};

/// Describes one paired device after merging inbound and outbound session records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePairedDevice {
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub outboundSessionName: Option<String>,
    pub outboundBaseUrl: Option<String>,
    pub outboundTransport: Option<LinkTransportPreference>,
    pub inboundSessionIds: Vec<String>,
}

/// Reports the remote identity returned after beginning an outbound pairing transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemotePairStartResult {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub coreUserName: String,
}

/// Describes a Link-enabled runtime discovered by the local runtime.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemoteDiscoveredDevice {
    pub deviceId: String,
    pub displayName: String,
    pub userName: String,
    pub platform: String,
    pub model: String,
    pub baseUrl: String,
    pub hostname: String,
    pub port: u16,
    pub tokenHash: String,
    pub version: String,
}

/// Groups every discovered CoreNode that currently advertises the same Space identity.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeRemoteDiscoveredSpace {
    pub spaceId: String,
    pub spaceName: String,
    pub spaceRevision: i64,
    pub memberCount: usize,
    pub devices: Vec<RuntimeRemoteDiscoveredDevice>,
}

/// Describes one device in the UI-facing device-space topology projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeviceSpaceDevice {
    pub deviceId: String,
    pub userName: String,
    pub deviceName: String,
    pub platform: String,
    pub model: String,
    pub coreVersion: Option<String>,
    pub online: bool,
}

/// Describes the current health state of one direct device-space connection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeDeviceSpaceConnectionStatus {
    Online,
    Offline,
    VersionMismatch,
    Unknown,
}

/// Describes one direct connection in the UI-facing device-space topology projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeviceSpaceConnection {
    pub firstDeviceId: String,
    pub secondDeviceId: String,
    pub status: RuntimeDeviceSpaceConnectionStatus,
    pub reason: String,
}

/// Describes the current device and all visible device-space connections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeviceSpaceTopology {
    pub currentDeviceId: String,
    pub devices: Vec<RuntimeDeviceSpaceDevice>,
    pub connections: Vec<RuntimeDeviceSpaceConnection>,
}

/// Provides runtime-owned remote session operations to generated local Core clients.
#[derive(Clone)]
pub struct RuntimeRemoteLinkService {
    localRuntime: Arc<CoreNodeLocalRuntime>,
    nodeRouter: CoreNodeRouter,
    linkAccessStore: LinkAccessStore,
    spaceStore: CoreSpaceStore,
}

impl RuntimeRemoteLinkService {
    /// Creates the service over the active local Core and its runtime-owned Link records.
    pub fn new(localRuntime: CoreNodeLocalRuntime) -> Self {
        let localRuntime = Arc::new(localRuntime);
        let linkAccessStore = LinkAccessStore::new(localRuntime.runtimeStorageHost());
        let spaceStore = CoreSpaceStore::new(localRuntime.runtimeStorageHost());
        let nodeRouter = CoreNodeRouter::new((*localRuntime).clone());
        Self {
            localRuntime,
            nodeRouter,
            linkAccessStore,
            spaceStore,
        }
    }

    /// Returns the converged Space membership owned by this CoreNode.
    #[allow(non_snake_case)]
    pub fn deviceSpace(&self) -> Result<CoreSpace, String> {
        self.spaceStore.initialize()
    }

    /// Returns the synchronized device metadata and direct-connection graph.
    #[allow(non_snake_case)]
    pub fn deviceSpaceTopology(&self) -> Result<RuntimeDeviceSpaceTopology, String> {
        let space = self.spaceStore.initialize()?;
        let profiles = self.spaceStore.deviceProfiles()?;
        let currentDeviceId = self.nodeRouter.localNodeId();
        let activePeers = activePeerNodeIds(&currentDeviceId)?;
        let devices = space
            .members
            .into_iter()
            .map(|deviceId| {
                let profile = profiles.get(&deviceId).ok_or_else(|| {
                    format!("Device profile is missing in the current device space: {deviceId}")
                })?;
                let online = deviceId == currentDeviceId
                    || self
                        .spaceStore
                        .reachableNextHopThroughPeers(deviceId.clone(), activePeers.clone())?
                        .is_some();
                Ok(runtimeDeviceSpaceDevice(profile, online))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let devicesById = devices
            .iter()
            .map(|device| (device.deviceId.clone(), device))
            .collect::<BTreeMap<_, _>>();
        let connections = self
            .spaceStore
            .deviceConnections()?
            .into_iter()
            .map(|connection| {
                let first = devicesById.get(&connection.firstDeviceId).ok_or_else(|| {
                    format!(
                        "Device profile is missing for connection endpoint: {}",
                        connection.firstDeviceId
                    )
                })?;
                let second = devicesById.get(&connection.secondDeviceId).ok_or_else(|| {
                    format!(
                        "Device profile is missing for connection endpoint: {}",
                        connection.secondDeviceId
                    )
                })?;
                let directlyOnline = if connection.firstDeviceId == currentDeviceId {
                    Some(activePeers.contains(&connection.secondDeviceId))
                } else if connection.secondDeviceId == currentDeviceId {
                    Some(activePeers.contains(&connection.firstDeviceId))
                } else {
                    None
                };
                let (status, reason) = runtimeDeviceSpaceConnectionState(
                    first,
                    second,
                    directlyOnline,
                );
                Ok(RuntimeDeviceSpaceConnection {
                    firstDeviceId: connection.firstDeviceId,
                    secondDeviceId: connection.secondDeviceId,
                    status,
                    reason,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(RuntimeDeviceSpaceTopology {
            currentDeviceId,
            devices,
            connections,
        })
    }

    /// Publishes the active user identity name as synchronized device metadata.
    #[allow(non_snake_case)]
    pub fn updateCurrentDeviceUserName(
        &self,
        userName: String,
    ) -> Result<RuntimeDeviceSpaceDevice, String> {
        let profile = self.spaceStore.writeLocalDeviceUserName(userName)?;
        Ok(runtimeDeviceSpaceDevice(&profile, true))
    }

    /// Adopts Space membership received through an explicit authenticated join.
    #[allow(non_snake_case)]
    pub fn adoptDeviceSpace(&self, space: CoreSpace) -> Result<CoreSpace, String> {
        self.spaceStore.adopt(space)
    }

    /// Records one directly paired device's current Space projection.
    #[allow(non_snake_case)]
    pub fn observePairedDeviceSpace(
        &self,
        deviceId: String,
        space: CoreSpace,
    ) -> Result<CoreSpace, String> {
        if !self.pairedDevicesSnapshot()?.contains_key(&deviceId) {
            return Err(format!("paired device does not exist: {deviceId}"));
        }
        self.spaceStore.observePairedDeviceSpace(deviceId, space)
    }

    /// Renames the current Space and returns its new synchronized identity.
    #[allow(non_snake_case)]
    pub fn renameDeviceSpace(&self, spaceName: String) -> Result<CoreSpace, String> {
        self.spaceStore.rename(spaceName)
    }

    /// Leaves the current device space while preserving all direct pairing records.
    #[allow(non_snake_case)]
    pub fn leaveDeviceSpace(&self) -> Result<CoreSpace, String> {
        let deviceSpace = self.spaceStore.leave()?;
        let memberNodeIds = deviceSpace.members.iter().cloned().collect::<BTreeSet<_>>();
        CoreNodeBindingStore::new(self.localRuntime.runtimeStorageHost())?
            .rebindOutsideNodesToLocal(&memberNodeIds)?;
        Ok(deviceSpace)
    }

    /// Joins the Space exposed by one directly paired CoreNode.
    #[allow(non_snake_case)]
    pub async fn joinPairedDeviceSpace(&self, name: String) -> Result<CoreSpace, String> {
        let (record, session) = self.pairedSession(&name)?;
        let info = session.sessionInfo().await?;
        ensureRemoteIdentity(&record, &info.coreDeviceId)?;
        let peerSpace: CoreSpace = serde_json::from_value(
            callRemoteService(
                &session,
                "runtimeRemoteLinkService",
                "deviceSpace",
                Value::Null,
            )
            .await?,
        )
        .map_err(|error| error.to_string())?;
        if !peerSpace
            .members
            .iter()
            .any(|nodeId| nodeId == &record.coreDeviceId)
        {
            return Err("paired device is not present in its advertised device space".to_string());
        }
        let merged = self.spaceStore.merge(peerSpace)?;
        let accepted: CoreSpace = serde_json::from_value(
            callRemoteService(
                &session,
                "runtimeRemoteLinkService",
                "adoptDeviceSpace",
                json!({ "space": merged }),
            )
            .await?,
        )
        .map_err(|error| error.to_string())?;
        self.spaceStore.adopt(accepted)?;
        self.persistenceSyncService()
            .synchronizePeer(name, 512, true)
            .await?;
        self.spaceStore.space()
    }

    /// Reads paired devices with inbound and outbound records merged by device id.
    #[allow(non_snake_case)]
    pub fn pairedDevicesSnapshot(&self) -> Result<BTreeMap<String, RuntimePairedDevice>, String> {
        mergePairedDevices(
            self.linkAccessStore.outboundSessions()?,
            self.linkAccessStore.inboundSessions()?,
        )
    }

    /// Observes paired devices after merging both connection directions by device id.
    #[allow(non_snake_case)]
    pub fn pairedDevicesFlow(
        &self,
    ) -> Result<StateFlow<BTreeMap<String, RuntimePairedDevice>>, String> {
        let inboundFlow = self.linkAccessStore.inboundSessionsFlow();
        let outboundFlow = self.linkAccessStore.outboundSessionsFlow();
        let initialInbound = inboundFlow.first().map_err(|error| error.to_string())?;
        let initialOutbound = outboundFlow.first().map_err(|error| error.to_string())?;
        mergePairedDevices(initialOutbound.clone(), initialInbound.clone())?;
        let inboundState =
            inboundFlow.stateIn(CoroutineScope, SharingStarted::Lazily, initialInbound);
        let outboundState =
            outboundFlow.stateIn(CoroutineScope, SharingStarted::Lazily, initialOutbound);
        Ok(combine2(
            &outboundState,
            &inboundState,
            |outbound, inbound| {
                mergePairedDevices(outbound, inbound)
                    .expect("validated Link Access session records must merge by device id")
            },
        ))
    }

    /// Returns whether one paired device currently has an active direct connection.
    #[allow(non_snake_case)]
    pub fn pairedDeviceOnline(&self, deviceId: String) -> Result<bool, String> {
        if !self.pairedDevicesSnapshot()?.contains_key(&deviceId) {
            return Err(format!("paired device does not exist: {deviceId}"));
        }
        isPeerLinkActive(&self.nodeRouter.localNodeId(), &deviceId)
    }

    /// Disconnects one directly adjacent device while preserving pairing records.
    #[allow(non_snake_case)]
    pub fn disconnectDeviceSpaceConnection(&self, deviceId: String) -> Result<(), String> {
        let localDeviceId = self.nodeRouter.localNodeId();
        let space = self.spaceStore.initialize()?;
        if !space.members.iter().any(|member| member == &deviceId) {
            return Err(format!("device is not a member of the current device space: {deviceId}"));
        }
        if deviceId == localDeviceId {
            return Err("current device cannot disconnect itself".to_string());
        }
        kickPeerLink(&localDeviceId, &deviceId)
    }

    /// Removes every local pairing record associated with one device.
    #[allow(non_snake_case)]
    pub fn removePairedDevice(&self, deviceId: String) -> Result<(), String> {
        let outboundNames = self
            .linkAccessStore
            .outboundSessions()?
            .into_iter()
            .filter(|(_, record)| record.coreDeviceId == deviceId)
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        let inboundSessionIds = self
            .linkAccessStore
            .inboundSessions()?
            .into_iter()
            .filter(|(_, record)| record.deviceId == deviceId)
            .map(|(sessionId, _)| sessionId)
            .collect::<Vec<_>>();
        if outboundNames.is_empty() && inboundSessionIds.is_empty() {
            return Err(format!("paired device does not exist: {deviceId}"));
        }
        disconnectPeerLink(&self.nodeRouter.localNodeId(), &deviceId)?;
        for name in outboundNames {
            self.linkAccessStore.removeOutboundSession(&name)?;
        }
        for sessionId in inboundSessionIds {
            self.linkAccessStore.removeInboundSession(&sessionId)?;
        }
        Ok(())
    }

    /// Starts the singleton persistent synchronization worker for direct Space peers.
    #[allow(non_snake_case)]
    pub fn startSpaceSync(&self) -> Result<(), String> {
        self.persistenceSyncService().start()
    }

    /// Discovers nearby Spaces and groups their directly connectable CoreNodes.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    pub async fn discoverSpaces(
        &self,
        timeoutMs: u64,
    ) -> Result<Vec<RuntimeRemoteDiscoveredSpace>, String> {
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
        self.refreshDiscoveredPairedRemoteEndpoints(&devices)
            .await?;
        self.groupDiscoveredSpaces(devices).await
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
        let identity = self.linkAccessStore.initializeIdentity(clientDeviceInfo)?;
        let state = client
            .pairStart(&tokenHash, identity.deviceId, identity.deviceInfo)
            .await?;
        if hello.coreDeviceId != state.coreDeviceId {
            return Err("paired remote identity changed during pairing".to_string());
        }
        self.linkAccessStore.savePendingOutboundPairing(
            state.pairingId.clone(),
            PendingOutboundPairingRecord {
                baseUrl,
                state: state.clone(),
            },
        )?;
        Ok(RuntimeRemotePairStartResult {
            pairingId: state.pairingId,
            pairingServiceVersion: state.pairingServiceVersion,
            coreDeviceId: state.coreDeviceId,
            coreDeviceInfo: state.coreDeviceInfo,
            coreUserName: hello.deviceSpace.userName,
        })
    }

    /// Completes a runtime-owned outbound pairing and stores its named direct connection.
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
            .removePendingOutboundPairing(&pairingId)?;
        Ok(record)
    }

    /// Persists the explicit carrier selected for one named outbound session.
    #[allow(non_snake_case)]
    pub fn setPairedRemoteTransport(
        &self,
        name: String,
        transport: LinkTransportPreference,
    ) -> Result<PairedRemoteSessionRecord, String> {
        let mut record = self
            .linkAccessStore
            .outboundSessions()?
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("paired remote runtime does not exist: {name}"))?;
        record.transport = transport;
        self.linkAccessStore
            .saveOutboundSession(name, record.clone())?;
        Ok(record)
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
        devices: &[RuntimeRemoteDiscoveryEndpoint],
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

    /// Resolves live Space identities for discovered devices and groups them by Space id.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    async fn groupDiscoveredSpaces(
        &self,
        devices: Vec<RuntimeRemoteDiscoveryEndpoint>,
    ) -> Result<Vec<RuntimeRemoteDiscoveredSpace>, String> {
        let mut spaces = BTreeMap::<String, RuntimeRemoteDiscoveredSpace>::new();
        for endpoint in devices {
            let hello = RemoteLinkClient::new(endpoint.baseUrl.clone())
                .hello(&endpoint.tokenHash)
                .await?;
            ensureRemoteIdentityById(&endpoint.deviceId, &hello.coreDeviceId)?;
            if hello.deviceSpace.deviceCount == 0 {
                return Err("discovered device space has no devices".to_string());
            }
            let device = RuntimeRemoteDiscoveredDevice {
                deviceId: endpoint.deviceId,
                displayName: hello.coreDeviceInfo.displayName(),
                userName: hello.deviceSpace.userName,
                platform: hello.coreDeviceInfo.platform,
                model: hello.coreDeviceInfo.model,
                baseUrl: endpoint.baseUrl,
                hostname: endpoint.hostname,
                port: endpoint.port,
                tokenHash: endpoint.tokenHash,
                version: endpoint.version,
            };
            let memberCount = hello.deviceSpace.deviceCount;
            match spaces.get_mut(&hello.deviceSpace.spaceId) {
                Some(space) => {
                    if hello.deviceSpace.spaceRevision > space.spaceRevision {
                        space.spaceName = hello.deviceSpace.spaceName.clone();
                        space.spaceRevision = hello.deviceSpace.spaceRevision;
                        space.memberCount = memberCount;
                    } else if hello.deviceSpace.spaceRevision == space.spaceRevision {
                        if hello.deviceSpace.spaceName != space.spaceName {
                            return Err(format!(
                                "device space {} advertises conflicting names at revision {}",
                                hello.deviceSpace.spaceId, hello.deviceSpace.spaceRevision
                            ));
                        }
                        space.memberCount = space.memberCount.max(memberCount);
                    }
                    space.devices.push(device);
                }
                None => {
                    spaces.insert(
                        hello.deviceSpace.spaceId.clone(),
                        RuntimeRemoteDiscoveredSpace {
                            spaceId: hello.deviceSpace.spaceId,
                            spaceName: hello.deviceSpace.spaceName,
                            spaceRevision: hello.deviceSpace.spaceRevision,
                            memberCount,
                            devices: vec![device],
                        },
                    );
                }
            }
        }
        for space in spaces.values_mut() {
            space.devices.sort_by(|left, right| {
                left.displayName
                    .cmp(&right.displayName)
                    .then(left.deviceId.cmp(&right.deviceId))
            });
        }
        Ok(spaces.into_values().collect())
    }

    /// Builds the persistent synchronization service owned by this runtime facade.
    #[allow(non_snake_case)]
    fn persistenceSyncService(&self) -> SpacePersistenceSyncService {
        SpacePersistenceSyncService::new(
            self.localRuntime.clone(),
            self.nodeRouter.clone(),
            self.linkAccessStore.clone(),
            self.spaceStore.clone(),
        )
    }
}

/// Converts one synchronized Store profile into the runtime-facing device model.
#[allow(non_snake_case)]
fn runtimeDeviceSpaceDevice(
    profile: &CoreSpaceDeviceProfile,
    online: bool,
) -> RuntimeDeviceSpaceDevice {
    RuntimeDeviceSpaceDevice {
        deviceId: profile.nodeId.clone(),
        userName: profile.userName.clone(),
        deviceName: profile.displayName.clone(),
        platform: profile.platform.clone(),
        model: profile.model.clone(),
        coreVersion: profile.coreVersion.clone(),
        online,
    }
}

/// Computes one connection status from both endpoint reachability and versions.
fn runtimeDeviceSpaceConnectionState(
    first: &RuntimeDeviceSpaceDevice,
    second: &RuntimeDeviceSpaceDevice,
    directlyOnline: Option<bool>,
) -> (RuntimeDeviceSpaceConnectionStatus, String) {
    let mut reasons = Vec::new();
    if !first.online {
        reasons.push(format!("{} is offline", first.deviceName));
    }
    if !second.online {
        reasons.push(format!("{} is offline", second.deviceName));
    }
    let versionsMismatch = match (&first.coreVersion, &second.coreVersion) {
        (Some(firstVersion), Some(secondVersion)) if firstVersion != secondVersion => {
            reasons.push(format!(
                "Core version mismatch: {}={}, {}={}",
                first.deviceName, firstVersion, second.deviceName, secondVersion
            ));
            true
        }
        _ => false,
    };
    if directlyOnline == Some(false) {
        reasons.push("Direct Peer Link is offline".to_string());
    }
    let status = if !first.online || !second.online || directlyOnline == Some(false) {
        RuntimeDeviceSpaceConnectionStatus::Offline
    } else if versionsMismatch {
        RuntimeDeviceSpaceConnectionStatus::VersionMismatch
    } else if first.coreVersion.is_none() || second.coreVersion.is_none() {
        reasons.push("Core version is unavailable".to_string());
        RuntimeDeviceSpaceConnectionStatus::Unknown
    } else if directlyOnline.is_none() {
        reasons.push(
            "Direct Peer Link status is not observable from the current device".to_string(),
        );
        RuntimeDeviceSpaceConnectionStatus::Unknown
    } else {
        RuntimeDeviceSpaceConnectionStatus::Online
    };
    let reason = if reasons.is_empty() {
        "Link is healthy".to_string()
    } else {
        reasons.join("; ")
    };
    (status, reason)
}

/// Merges inbound and outbound pairing records into one device-indexed projection.
#[allow(non_snake_case)]
fn mergePairedDevices(
    outboundSessions: BTreeMap<String, PairedRemoteSessionRecord>,
    inboundSessions: BTreeMap<String, AcceptedRemoteSessionRecord>,
) -> Result<BTreeMap<String, RuntimePairedDevice>, String> {
    let mut devices = BTreeMap::<String, RuntimePairedDevice>::new();
    for (sessionName, record) in outboundSessions {
        let device = devices
            .entry(record.coreDeviceId.clone())
            .or_insert_with(|| RuntimePairedDevice {
                deviceId: record.coreDeviceId.clone(),
                deviceInfo: record.remoteDeviceInfo.clone(),
                outboundSessionName: None,
                outboundBaseUrl: None,
                outboundTransport: None,
                inboundSessionIds: Vec::new(),
            });
        ensureDeviceInfoMatches(&device.deviceInfo, &record.remoteDeviceInfo)?;
        if device.outboundSessionName.is_some() {
            return Err(format!(
                "multiple outgoing pairings target device {}",
                record.coreDeviceId
            ));
        }
        device.outboundSessionName = Some(sessionName);
        device.outboundBaseUrl = Some(record.baseUrl);
        device.outboundTransport = Some(record.transport);
    }
    for (sessionId, record) in inboundSessions {
        let device =
            devices
                .entry(record.deviceId.clone())
                .or_insert_with(|| RuntimePairedDevice {
                    deviceId: record.deviceId.clone(),
                    deviceInfo: record.deviceInfo.clone(),
                    outboundSessionName: None,
                    outboundBaseUrl: None,
                    outboundTransport: None,
                    inboundSessionIds: Vec::new(),
                });
        ensureDeviceInfoMatches(&device.deviceInfo, &record.deviceInfo)?;
        device.inboundSessionIds.push(sessionId);
    }
    Ok(devices)
}

/// Invokes one generated service method through an authenticated paired remote session.
#[allow(non_snake_case)]
async fn callRemoteService(
    session: &PairedRemoteSession,
    targetPath: &str,
    methodName: &str,
    args: Value,
) -> Result<Value, String> {
    let response = session
        .call(serviceCallRequest(targetPath, methodName, args)?)
        .await?;
    coreResponseValue(response.result.map_err(|error| error.to_string())?)
}

/// Builds one Link request for a generated Core service operation.
#[allow(non_snake_case)]
fn serviceCallRequest(
    targetPath: &str,
    methodName: &str,
    args: Value,
) -> Result<CoreCallRequest, String> {
    let targetObjectId = crate::spaceObjectIdForSchema(targetPath)
        .ok_or_else(|| format!("unknown Core schema key: {targetPath}"))?;
    Ok(CoreCallRequest::new(
        format!("runtime-remote-{methodName}-{}", currentTimeMillis()),
        targetObjectId,
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

/// Verifies one observed CoreNode id against an authenticated Link response.
#[allow(non_snake_case)]
fn ensureRemoteIdentityById(expectedNodeId: &str, observedNodeId: &str) -> Result<(), String> {
    if observedNodeId != expectedNodeId {
        return Err(format!(
            "paired device identity mismatch: expected={}, observed={observedNodeId}",
            expectedNodeId
        ));
    }
    Ok(())
}

/// Verifies that directional session records describe the same paired device.
#[allow(non_snake_case)]
fn ensureDeviceInfoMatches(
    expected: &RemoteDeviceInfo,
    observed: &RemoteDeviceInfo,
) -> Result<(), String> {
    if expected.platform != observed.platform || expected.model != observed.model {
        return Err("paired device metadata conflicts across session directions".to_string());
    }
    Ok(())
}
