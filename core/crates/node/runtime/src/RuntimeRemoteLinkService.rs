use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::protocol::LinkDeviceInfo;
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreValue, CORE_INTERNAL_TARGET};
use operit_store::CoreNodeBindingStore::CoreNodeBindingStore;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile, CoreSpaceStore, CoreSpaceTopologyRecord};
use operit_store::NetworkControlStore::{
    NetworkControlAuditRecord, NetworkControlIdentityAssignment, NetworkControlRole,
    NetworkControlState, NetworkControlStore,
};
use operit_store::PreferencesDataStore::StateFlow;
use operit_store::SyncOperationStore::{subscribeSyncMutations, SyncOperation};
use operit_tools::runtime_support::CoreRouteResumeContext;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::{
    CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter},
    GeneratedRouteLifecycle,
    NodeServices::NodeServices,
    PeerSync::PeerSyncMethod,
    SpacePersistenceSyncService::SpacePersistenceSyncService,
};

/// Runtime 的 Space 业务对象；仍使用标准 Link Call，不新增握手消息或 HTTP 路径。
pub(crate) const NODE_SPACE_TARGET: &str = "node.space";
// Same-Space, authenticated routing only; never available to an unadmitted applicant.
pub(crate) const NODE_SPACE_APPROVAL_TARGET: &str = "node.space.approval";

/// Carries a complete Space projection and the policy that authorizes its members.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PeerSpaceSnapshot {
    pub(crate) space: CoreSpace,
    pub(crate) deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub(crate) controlOperations: Vec<SyncOperation>,
    pub(crate) topology: Vec<CoreSpaceTopologyRecord>,
}

type PeerSpaceJoin = PeerSpaceSnapshot;

/// Join approval is separate from pairing and from synchronized membership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpaceJoinStatus { Pending, Approving, Approved, Rejected, Cancelled, Expired, Joined }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceJoinRequest {
    pub requestId: String,
    pub targetDeviceId: String,
    pub applicantDeviceId: String,
    pub applicantName: String,
    pub spaceName: String,
    pub status: SpaceJoinStatus,
    pub createdAt: i64,
    pub expiresAt: i64,
    pub canApprove: bool,
    #[serde(default)]
    pub reviewerDeviceId: Option<String>,
    #[serde(default)]
    pub reviewerName: Option<String>,
    #[serde(default)]
    pub reviewerHops: Option<u32>,
    #[serde(default)]
    pub assignmentVersion: u64,
    #[serde(default)]
    pub decisionApprove: Option<bool>,
}

#[path = "peer/space_join.rs"]
mod space_join;

/// 已配对设备的展示投影；不暴露底层会话、端点或传输选择。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePairedDevice {
    pub deviceId: String,
    pub deviceInfo: LinkDeviceInfo,
    pub inbound: bool,
    pub outbound: bool,
}

/// Reports whether one persisted pairing is usable by the current Space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimePairedDeviceStatus {
    Online,
    Offline,
    Invalid,
    RemovedFromSpace,
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
    pub currentIdentity: Option<RuntimeDeviceSpaceIdentity>,
}

/// Describes the identity and effective capabilities currently assigned to one device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeviceSpaceIdentity {
    pub displayName: String,
    pub capabilities: Vec<String>,
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

/// Keeps the overview membership and topology in one observable UI snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDeviceSpaceSnapshot {
    pub space: CoreSpace,
    pub topology: RuntimeDeviceSpaceTopology,
}

/// 类型化 Proxy 的本机管理适配入口，以及 Space/network-control 业务服务。
/// 配对、发现和监听只委托同一个 RuntimePeerService；不在这里实现第二套流程。
/// It does not own sockets, pairing sessions, discovery clients, or transport managers.
#[derive(Clone)]
pub struct RuntimeRemoteLinkService {
    localRuntime: Arc<CoreNodeLocalRuntime>,
    nodeRouter: CoreNodeRouter,
    spaceStore: CoreSpaceStore,
    networkControlStore: NetworkControlStore,
    persistenceSync: SpacePersistenceSyncService,
}

impl RuntimeRemoteLinkService {
    /// Creates the Space business facade. Connectivity is injected separately.
    pub fn new(localRuntime: CoreNodeLocalRuntime) -> Self {
        let nodeRouter = CoreNodeRouter::new(localRuntime.clone());
        Self::newWithRouter(localRuntime, nodeRouter)
    }

    /// Creates the facade using the router owned by the application tree.
    #[allow(non_snake_case)]
    pub fn newWithRouter(localRuntime: CoreNodeLocalRuntime, nodeRouter: CoreNodeRouter) -> Self {
        let localRuntime = Arc::new(localRuntime);
        let spaceStore = CoreSpaceStore::new(localRuntime.runtimeStorageHost());
        let networkControlStore = NetworkControlStore::new(localRuntime.runtimeStorageHost())
            .expect("RuntimeRemoteLinkService requires network control storage");
        let persistenceSync = SpacePersistenceSyncService::new(
            localRuntime.clone(),
            nodeRouter.clone(),
            spaceStore.clone(),
        );
        Self {
            localRuntime,
            nodeRouter,
            spaceStore,
            networkControlStore,
            persistenceSync,
        }
    }

    fn nodeServices(&self) -> Result<&NodeServices, String> {
        self.nodeRouter.nodeServices()
    }

    /// Exposes the Core's unpaired discovery candidates uniformly through generated application proxies.
    pub async fn discoverPeers(&self, timeoutMs: u64) -> Result<Vec<crate::NodeServices::DiscoveredPeer>, String> {
        self.nodeServices()?.peers().discoverPeers(timeoutMs).await.map_err(|error| error.to_string())
    }
    pub async fn startPairing(&self, nodeId: String, address: String,
        transport: operit_peer_link::PeerTransport, token: Option<String>,
    ) -> Result<crate::NodeServices::PendingPairing, String> {
        self.nodeServices()?.peers().startPairing(crate::NodeServices::PeerEndpoint { nodeId, address },
            transport, token.as_deref()).await.map_err(|error| error.to_string())
    }
    pub async fn finishPairing(&self, pairingId: String, confirmationCode: String)
        -> Result<crate::NodeServices::PairedPeer, String> {
        self.nodeServices()?.peers().finishPairing(&pairingId, &confirmationCode).await.map_err(|error| error.to_string())
    }
    pub async fn cancelPairing(&self, pairingId: String) -> Result<(), String> {
        self.nodeServices()?.peers().cancelPairing(&pairingId).await.map_err(|error| error.to_string())
    }
    pub fn pairingPrompts(&self) -> Result<Vec<crate::NodeServices::PairingPrompt>, String> {
        self.nodeServices()?.peers().pairingPrompts().map_err(|error| error.to_string())
    }
    /// 仅本机 UI 观察待确认配对；使用共享节点状态通知，不建立第二套事件通道。
    pub fn pairingPromptsFlow(&self) -> Result<StateFlow<Vec<crate::NodeServices::PairingPrompt>>, String> {
        self.observePeerState(Self::pairingPrompts)
    }
    /// Exposes independent listener capabilities from the injected node peer service.
    pub fn listenerCapabilities(&self) -> Result<operit_peer_link::PeerListenerCapabilities, String> {
        Ok(self.nodeServices()?.peers().listenerCapabilities())
    }

    /// Opens only explicitly requested transports after runtime capability validation.
    pub async fn startListening(&self, transports: Vec<operit_peer_link::PeerTransport>) -> Result<(), String> {
        self.nodeServices()?.peers().startListening(&transports).await.map_err(|error| error.to_string())
    }
    pub async fn stopListening(&self) -> Result<(), String> {
        self.nodeServices()?.peers().stop().await.map_err(|error| error.to_string())
    }
    /// 本机配置继续读取原路径；不生成另一份 token 或监听配置。
    pub fn localHostConfig(&self) -> Result<Option<crate::PeerStateStore::PeerHostConfig>, String> {
        crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost()).hostConfig()
    }
    pub fn saveLocalHostConfig(&self, config: crate::PeerStateStore::PeerHostConfig) -> Result<(), String> {
        crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost()).saveHostConfig(&config)
    }
    /// 仅本机用户显式查看/复制，远端路由不允许调用。
    pub fn localPairingToken(&self) -> Result<String, String> {
        crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost()).localPairingToken()
    }
    /// Rotates the node-local listener credential through its preference transaction.
    pub fn refreshLocalPairingToken(&self) -> Result<String, String> {
        crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost())
            .refreshLocalPairingToken()
    }
    /// 复用原身份文件并刷新 Space 资料；通信会话不参与设备资料初始化。
    pub fn initializeDeviceInfo(&self, supplied: LinkDeviceInfo) -> Result<LinkDeviceInfo, String> {
        let info = crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost())
            .deviceInfo(supplied, false)?;
        self.writeDeviceProfile(&info)?;
        self.networkControlStore.initializeCurrentSpace()?;
        Ok(info)
    }

    pub fn updateDeviceInfo(&self, supplied: LinkDeviceInfo) -> Result<LinkDeviceInfo, String> {
        let info = crate::PeerStateStore::PeerStateStore::new(self.localRuntime.runtimeStorageHost())
            .deviceInfo(supplied, true)?;
        self.writeDeviceProfile(&info)?;
        Ok(info)
    }

    fn writeDeviceProfile(&self, info: &LinkDeviceInfo) -> Result<(), String> {
        self.spaceStore.writeLocalDeviceProfile(info.displayName(), info.platform.clone(),
            info.model.clone(), operit_runtime::CORE_VERSION.to_string()).map(|_| ())
    }

    /// Returns the converged Space membership owned by this CoreNode.
    #[allow(non_snake_case)]
    pub fn deviceSpace(&self) -> Result<CoreSpace, String> {
        self.spaceStore.initialize()
    }

    /// Reads a complete overview, retrying if membership changes during the read.
    pub fn deviceSpaceSnapshot(&self) -> Result<RuntimeDeviceSpaceSnapshot, String> {
        for _ in 0..3 {
            let space = self.deviceSpace()?;
            let topology = self.deviceSpaceTopology()?;
            if space == self.deviceSpace()?
                && space.members.iter().collect::<BTreeSet<_>>()
                    == topology
                        .devices
                        .iter()
                        .map(|device| &device.deviceId)
                        .collect()
            {
                return Ok(RuntimeDeviceSpaceSnapshot { space, topology });
            }
        }
        Err("Device space changed while reading its overview".to_string())
    }

    /// Observes persistent Space changes and live Peer Links through the shared Host scheduler.
    pub fn deviceSpaceSnapshotFlow(&self) -> Result<StateFlow<RuntimeDeviceSpaceSnapshot>, String> {
        let (changes, mut changed) = tokio::sync::mpsc::channel(1);
        let mutationSubscription = subscribeSyncMutations(move || {
            let _ = changes.try_send(());
        });
        let mut peers = self.nodeServices()?.peers().subscribePeerChanges();
        let state = StateFlow::new(self.deviceSpaceSnapshot()?);
        let service = self.clone();
        let (stop, mut stopped) = oneshot::channel::<()>();
        let overview = spaceOverviewSubscription(&state, stop);
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "device-space-overview-watch",
            Box::new(move || Box::pin(async move {
                let _subscription = mutationSubscription;
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        event = changed.recv() => { if event.is_none() { break; } },
                        event = peers.recv() => {
                            if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; }
                        },
                    }
                    // Join writes several records. Coalesce the burst and let writers
                    // release their datastore locks before reading the projection.
                    tokio::select! {
                        _ = &mut stopped => break,
                        result = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(25) => {
                            if let Err(error) = result {
                                operit_util::AppLogger::AppLogger::w(
                                    "RuntimeRemoteLinkService", &format!("Space overview delay failed: {error}"));
                                break;
                            }
                        },
                    }
                    match service.deviceSpaceSnapshot() {
                        Ok(snapshot) => state.set_value(snapshot),
                        Err(error) => { operit_util::AppLogger::AppLogger::w(
                            "RuntimeRemoteLinkService", &format!("Space overview refresh failed: {error}")); },
                    }
                }
            })),
        ).map_err(|error| error.to_string())?;
        Ok(overview)
    }

    /// Creates the initial administrator policy for this device's new single-device Space.
    #[allow(non_snake_case)]
    pub fn bootstrapDeviceSpaceControl(&self) -> Result<NetworkControlState, String> {
        self.networkControlStore.bootstrapCurrentSpace()
    }

    /// Returns current identity definitions, device assignments, and removal state.
    #[allow(non_snake_case)]
    pub fn deviceSpaceControl(&self) -> Result<NetworkControlState, String> {
        self.networkControlStore.currentState()
    }

    /// Returns accepted and rejected authorization commands for the current Space.
    #[allow(non_snake_case)]
    pub fn deviceSpaceControlAudit(&self) -> Result<Vec<NetworkControlAuditRecord>, String> {
        let localNodeId = self.nodeRouter.localNodeId();
        if !self
            .networkControlStore
            .nodeHasCapability(&localNodeId, "network.audit.read", None)?
        {
            return Err("current device cannot read the Space control audit".to_string());
        }
        self.networkControlStore.audit()
    }

    /// Defines one custom role from named capabilities.
    #[allow(non_snake_case)]
    pub fn defineDeviceSpaceRole(&self, role: NetworkControlRole) -> Result<(), String> {
        self.networkControlStore.defineRole(role).map(|_| ())
    }

    /// Sets one existing identity as the device's current identity.
    #[allow(non_snake_case)]
    pub fn setDeviceSpaceIdentity(
        &self,
        assignment: NetworkControlIdentityAssignment,
    ) -> Result<(), String> {
        self.networkControlStore.setIdentity(assignment).map(|_| ())
    }

    /// Resets one device's identity to the default user identity. Members
    /// keep an identity at all times, so a reset never strips the basic
    /// capabilities the device's own UI depends on.
    #[allow(non_snake_case)]
    pub fn clearDeviceSpaceIdentity(&self, nodeId: String) -> Result<(), String> {
        self.networkControlStore.clearIdentity(nodeId).map(|_| ())
    }

    /// Updates one named Space policy setting under the current administrator policy.
    #[allow(non_snake_case)]
    pub fn updateDeviceSpacePolicy(&self, policyId: String, value: String) -> Result<(), String> {
        self.networkControlStore
            .updatePolicy(policyId, value)
            .map(|_| ())
    }

    /// Restores an existing Space member to ordinary membership before it reconnects or rejoins.
    #[allow(non_snake_case)]
    pub fn admitDeviceSpaceMember(&self, deviceId: String) -> Result<(), String> {
        self.networkControlStore.admitMember(deviceId).map(|_| ())
    }

    /// Forgets a device completely: the Space policy drops every record for it,
    /// the Space membership ejects it, its join-request history is deleted, and
    /// every local pairing credential is removed while its live Peer Link ends.
    /// The device returns to stranger state — reconnecting requires a fresh
    /// pairing plus a new join approval.
    #[allow(non_snake_case)]
    pub async fn removeDeviceSpaceMember(&self, deviceId: String) -> Result<(), String> {
        self.networkControlStore.removeMember(deviceId.clone())?;
        self.spaceStore.removeRemoteMember(deviceId.clone())?;
        space_join::purgeDevice(self, &deviceId)?;
        self.nodeServices()?.peers().removePairedPeer(&deviceId).await
            .map_err(|error| error.to_string())
    }

    /// Prohibits one device from direct connection and route transit immediately.
    #[allow(non_snake_case)]
    pub async fn disconnectDeviceSpaceNode(&self, deviceId: String) -> Result<(), String> {
        self.networkControlStore.disconnectNode(deviceId.clone())?;
        self.nodeServices()?.peers().disconnectPeer(&deviceId).await
            .map_err(|error| error.to_string())
    }

    /// Returns the synchronized device metadata and direct-connection graph.
    #[allow(non_snake_case)]
    pub fn deviceSpaceTopology(&self) -> Result<RuntimeDeviceSpaceTopology, String> {
        let space = self.spaceStore.initialize()?;
        let controlState = self.networkControlStore.currentState()?;
        let profiles = self.spaceStore.deviceProfiles()?;
        let currentDeviceId = self.nodeRouter.localNodeId();
        let activePeers = self
            .nodeServices()?.peers()
            .activePeerNodeIds()
            .map_err(|error| error.to_string())?;
        let devices = space
            .members
            .iter().cloned()
            .map(|deviceId| {
                let profile = profiles.get(&deviceId).ok_or_else(|| {
                    format!("Device profile is missing in the current device space: {deviceId}")
                })?;
                let online =
                    deviceId == currentDeviceId || self.nodeRouter.nodeIsReachable(&deviceId)?;
                Ok(runtimeDeviceSpaceDevice(
                    profile,
                    online,
                    runtimeDeviceSpaceIdentity(&controlState, &deviceId),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let devicesById = devices
            .iter()
            .map(|device| (device.deviceId.clone(), device))
            .collect::<BTreeMap<_, _>>();
        let connections = self
            .spaceStore
            .deviceConnectionsForSpace(&space)?
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
                let (status, reason) =
                    runtimeDeviceSpaceConnectionState(first, second, directlyOnline);
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
        let controlState = self.networkControlStore.currentState()?;
        Ok(runtimeDeviceSpaceDevice(
            &profile,
            true,
            runtimeDeviceSpaceIdentity(&controlState, &profile.nodeId),
        ))
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
        let space = self.spaceStore.leave()?;
        // Leaving creates a new singleton Space. Its creator must receive the
        // initial policy here, not only on the next application startup.
        self.networkControlStore.initializeCurrentSpace()?;
        Ok(space)
    }

    /// 加入直接配对节点的 Space。地址、会话与鉴权由同一个节点通信服务处理。
    pub async fn joinPairedDeviceSpace(&self, deviceId: String) -> Result<CoreSpace, String> {
        // Compatibility entry point: never admit a member without local approval.
        let request = self.requestDeviceSpaceJoin(deviceId).await?;
        let status = self.refreshDeviceSpaceJoin(request.requestId).await?;
        if status.status != SpaceJoinStatus::Joined {
            return Err("SPACE_JOIN_PENDING: Waiting for the target device to approve the join request".into());
        }
        self.spaceStore.space()
    }

    pub async fn requestDeviceSpaceJoin(&self, deviceId: String) -> Result<SpaceJoinRequest, String> {
        space_join::request(self, deviceId).await
    }

    pub fn outgoingDeviceSpaceJoins(&self) -> Result<Vec<SpaceJoinRequest>, String> {
        space_join::outgoing(self)
    }

    pub async fn incomingDeviceSpaceJoins(&self) -> Result<Vec<SpaceJoinRequest>, String> {
        space_join::incoming(self).await
    }

    pub async fn refreshDeviceSpaceJoin(&self, requestId: String) -> Result<SpaceJoinRequest, String> {
        space_join::refresh(self, requestId).await
    }

    pub async fn decideDeviceSpaceJoin(&self, requestId: String, assignmentVersion: u64, approve: bool) -> Result<SpaceJoinRequest, String> {
        space_join::decide(self, requestId, assignmentVersion, approve).await
    }

    pub async fn cancelDeviceSpaceJoin(&self, requestId: String) -> Result<SpaceJoinRequest, String> {
        space_join::cancel(self, requestId).await
    }

    async fn callPeerSpace<T: serde::de::DeserializeOwned>(
        &self, deviceId: &str, method: &str, args: CoreValue,
    ) -> Result<T, String> {
        let response = self.nodeRouter.callNode(deviceId.to_string(), CoreCallRequest::new(
            format!("node-space-{method}-{}", currentTimeMillis()),
            NODE_SPACE_TARGET, method, args,
        )).await;
        fromCoreValue(response.result.map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Captures member profiles and control history for direct Space propagation.
    pub(crate) fn peerSpaceSnapshot(&self) -> Result<PeerSpaceSnapshot, String> {
        let snapshot = PeerSpaceSnapshot {
            space: self.deviceSpace()?,
            deviceProfiles: self.spaceStore.deviceProfiles()?.into_values().collect(),
            controlOperations: self.networkControlStore.currentSpaceOperations()?,
            topology: self.spaceStore.topologyRecords()?.into_values().collect(),
        };
        CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
        Ok(snapshot)
    }

    /// Applies a complete peer projection, following only an approved cross-Space merge.
    pub(crate) fn observePeerSpaceSnapshot(&self, peerNodeId: &str, snapshot: PeerSpaceSnapshot) -> Result<CoreSpace, String> {
        CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
        if !snapshot.space.members.iter().any(|node| node == peerNodeId) {
            return Err("Paired device is not present in its announced device space".into());
        }
        let local = self.deviceSpace()?;
        let crossing = local.spaceId != snapshot.space.spaceId;
        if crossing {
            let localOperations = self.networkControlStore.currentSpaceOperations()?;
            let localAdmittedSource = localOperations.iter().map(|operation| {
                serde_json::from_value::<operit_store::NetworkControlStore::NetworkControlCommandRecord>(operation.payload.clone())
                    .map_err(|error| error.to_string())
            }).collect::<Result<Vec<_>, _>>()?.iter().any(|record| {
                record.spaceId == local.spaceId && matches!(&record.command,
                    operit_store::NetworkControlStore::NetworkControlCommand::AdmitSpace { sourceSpaceId, .. }
                    if sourceSpaceId == &snapshot.space.spaceId)
            });
            if localAdmittedSource {
                self.networkControlStore.validateSpaceAdmission(&local.spaceId, &snapshot.space.spaceId,
                    &snapshot.space.members.iter().cloned().collect(), &localOperations)?;
                // This endpoint has already migrated; publish its target projection to the source peer.
                self.spaceStore.importDeviceProfiles(snapshot.deviceProfiles)?;
                return Ok(local);
            }
            if !snapshot.space.members.iter().any(|node| node == &self.nodeRouter.localNodeId()) {
                // Independent paired Spaces do not merge without a target-side admission.
                return Ok(local);
            }
            if !local.members.iter().any(|node| node == peerNodeId) {
                return Err("Space migration must arrive through an existing source Space member".into());
            }
            self.networkControlStore.validateSpaceAdmission(&snapshot.space.spaceId,
                &local.spaceId, &local.members.iter().cloned().collect(), &snapshot.controlOperations)?;
        }
        let policy = self.networkControlStore.validateSpacePolicy(&snapshot.space.spaceId, &snapshot.controlOperations)?;
        for node in &snapshot.space.members {
            if !policy.memberNodeIds.contains(node) {
                return Err(format!("Space snapshot contains an unauthorized member: {node}"));
            }
        }
        // Profiles and authorization must be available before any member reference is published.
        self.spaceStore.importDeviceProfiles(snapshot.deviceProfiles)?;
        self.spaceStore.importTopologyRecords(snapshot.topology)?;
        for operation in &snapshot.controlOperations {
            self.networkControlStore.applyBootstrapOperation(operation)?;
        }
        if crossing {
            self.spaceStore.adopt(snapshot.space)
        } else {
            self.spaceStore.observePairedDeviceSpace(peerNodeId.to_string(), snapshot.space)
        }
    }

    /// 仅 Router 验证直接入站授权后调用；身份来自已鉴权连接，不从 args 取身份。
    pub(crate) fn acceptPeerSpaceCall(
        &self, peerNodeId: &str, request: CoreCallRequest,
    ) -> Result<CoreValue, String> {
        match request.methodName.as_str() {
            "snapshot" => toCoreValue(self.peerSpaceSnapshot()?).map_err(|error| error.to_string()),
            "observeSpaceSnapshot" => {
                let snapshot: PeerSpaceSnapshot = fromCoreValue(request.args).map_err(|error| error.to_string())?;
                toCoreValue(self.observePeerSpaceSnapshot(peerNodeId, snapshot)?)
                    .map_err(|error| error.to_string())
            },
            "deviceSpace" => toCoreValue(self.deviceSpace()?).map_err(|error| error.to_string()),
            "observePairedDeviceSpace" => {
                let space: CoreSpace = fromCoreValue(request.args).map_err(|error| error.to_string())?;
                toCoreValue(self.spaceStore.observePairedDeviceSpace(peerNodeId.to_string(), space)?)
                    .map_err(|error| error.to_string())
            },
            "requestJoin" | "joinStatus" | "cancelJoin" => space_join::receive(self, peerNodeId, request),
            "join" => Err("SPACE_JOIN_APPROVAL_REQUIRED: Submit a join request for local approval first".into()),
            _ => Err("Unknown node Space method".into()),
        }
    }

    pub(crate) async fn acceptSpaceApprovalCall(&self, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
        space_join::receiveApproval(self, origin, request)
    }

    /// Returns one device-indexed projection from the preserved state files, without exposing sessions.
    #[allow(non_snake_case)]
    pub fn pairedDevicesSnapshot(&self) -> Result<BTreeMap<String, RuntimePairedDevice>, String> {
        let peers = self
            .nodeServices()?.peers()
            .pairedPeers()
            .map_err(|error| error.to_string())?;
        Ok(peers
            .into_iter()
            .map(|peer| {
                (
                    peer.nodeId.clone(),
                    RuntimePairedDevice {
                        deviceId: peer.nodeId,
                        deviceInfo: LinkDeviceInfo {
                            platform: String::new(),
                            model: peer.displayName.clone(),
                        },
                        inbound: peer.inbound,
                        outbound: peer.outbound,
                    },
                )
            })
            .collect())
    }

    /// Connectivity and pairing state are exposed by the shared NodeServices instance.
    #[allow(non_snake_case)]
    pub fn pairedDevicesFlow(
        &self,
    ) -> Result<StateFlow<BTreeMap<String, RuntimePairedDevice>>, String> {
        self.observePeerState(|service| service.pairedDevicesSnapshot())
    }

    #[allow(non_snake_case)]
    pub fn pairedDeviceStatusesFlow(
        &self,
    ) -> Result<StateFlow<BTreeMap<String, RuntimePairedDeviceStatus>>, String> {
        self.observePeerState(|service| {
            Ok(pairedDeviceStatusesFromState(
                service.pairedDevicesSnapshot()?,
                service.nodeServices()?.peers().activePeerNodeIds()
                    .map_err(|error| error.to_string())?,
            ))
        })
    }

    /// 先订阅再取快照；断线、配对和撤销都更新观察值，最后一个订阅释放时退出。
    fn observePeerState<T>(
        &self,
        snapshot: fn(&Self) -> Result<T, String>,
    ) -> Result<StateFlow<T>, String>
    where
        T: Clone + PartialEq + Send + 'static,
    {
        let mut changes = self.nodeServices()?.peers().subscribePeerChanges();
        let state = StateFlow::new(snapshot(self)?);
        let (stop, mut stopped) = oneshot::channel();
        let watch = spaceOverviewSubscription(&state, stop);
        let service = self.clone();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "node-peer-state-watch",
            Box::new(move || Box::pin(async move {
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        event = changes.recv() => {
                            if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                                break;
                            }
                        },
                    }
                    match snapshot(&service) {
                        Ok(value) => state.set_value(value),
                        Err(error) => {
                            operit_util::AppLogger::AppLogger::w(
                                "RuntimeRemoteLinkService", &format!("Peer state refresh failed: {error}"));
                        },
                    }
                }
            })),
        ).map_err(|error| error.to_string())?;
        Ok(watch)
    }

    /// Returns whether one paired device currently has an active Peer Link.
    #[allow(non_snake_case)]
    pub fn pairedDeviceOnline(&self, deviceId: String) -> Result<bool, String> {
        if !self.pairedDevicesSnapshot()?.contains_key(&deviceId) {
            return Err(format!("paired device does not exist: {deviceId}"));
        }
        Ok(self.nodeServices()?.peers().activePeerNodeIds()
            .map_err(|error| error.to_string())?.contains(&deviceId))
    }

    /// Commits a chat Binding change, installs it on the target Core, and resumes there.
    #[allow(non_snake_case)]
    pub async fn requestChangeRoute(
        &self,
        chatId: String,
        targetNodeId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> Result<(), String> {
        if chatId.trim().is_empty() {
            return Err("route change chat id must not be empty".to_string());
        }
        if targetNodeId.trim().is_empty() {
            return Err("route change target node id must not be empty".to_string());
        }
        let space = self.spaceStore.initialize()?;
        if !space.members.iter().any(|member| member == &targetNodeId) {
            return Err(format!(
                "route change target is not a member of the current device space: {targetNodeId}"
            ));
        }
        if !self
            .networkControlStore
            .nodeHasCapability(&targetNodeId, "runtime.execute", None)?
        {
            return Err(format!(
                "route change target cannot execute runtime work: {targetNodeId}"
            ));
        }
        let localNodeId = self.nodeRouter.localNodeId();
        if targetNodeId != localNodeId && !self.nodeRouter.nodeIsReachable(&targetNodeId)? {
            return Err(format!(
                "route change target is not reachable in the current device space: {targetNodeId}"
            ));
        }
        self.callChatCoreLifecycle(
            &localNodeId,
            GeneratedRouteLifecycle::BeforeChangeRoute,
            chatId.clone(),
            None,
        )
        .await?;

        let routeChangeStartedAt = currentTimeMillis();
        let bindingStore = CoreNodeBindingStore::new(self.localRuntime.runtimeStorageHost())?;
        let currentBinding = bindingStore.binding(&chatId)?;
        operit_util::AppLogger::AppLogger::trace(
            "CoreRouteTrace",
            &format!(
                "route_change.binding_observed chatId={} local={} currentOwner={} currentGeneration={} target={} elapsedMs={}",
                chatId,
                localNodeId,
                currentBinding.nodeId,
                currentBinding.generation,
                targetNodeId,
                currentTimeMillis() - routeChangeStartedAt
            ),
        );
        if currentBinding.nodeId == targetNodeId {
            operit_util::AppLogger::AppLogger::trace(
                "CoreRouteTrace",
                &format!(
                    "route_change.noop chatId={} owner={} generation={} elapsedMs={}",
                    chatId,
                    currentBinding.nodeId,
                    currentBinding.generation,
                    currentTimeMillis() - routeChangeStartedAt
                ),
            );
        }

        if targetNodeId != localNodeId {
            operit_util::AppLogger::AppLogger::trace(
                "CoreRouteTrace",
                &format!(
                    "route_change.sync_before_commit.start chatId={} fromOwner={} target={} generation={} elapsedMs={}",
                    chatId,
                    currentBinding.nodeId,
                    targetNodeId,
                    currentBinding.generation,
                    currentTimeMillis() - routeChangeStartedAt
                ),
            );
            self.persistenceSyncService()
                .synchronizeReachablePeer(targetNodeId.clone(), 512, false)
                .await?;
            operit_util::AppLogger::AppLogger::trace(
                "CoreRouteTrace",
                &format!(
                    "route_change.sync_before_commit.done chatId={} fromOwner={} target={} generation={} elapsedMs={}",
                    chatId,
                    currentBinding.nodeId,
                    targetNodeId,
                    currentBinding.generation,
                    currentTimeMillis() - routeChangeStartedAt
                ),
            );
        }

        let commit = bindingStore.compareAndSet(&chatId, &currentBinding.nodeId, &targetNodeId)?;
        if commit.binding.nodeId != targetNodeId {
            return Err(format!(
                "route change committed to an unexpected target: {}",
                commit.binding.nodeId
            ));
        }
        operit_util::AppLogger::AppLogger::trace(
            "CoreRouteTrace",
            &format!(
                "route_change.binding_committed chatId={} fromOwner={} target={} generation={} elapsedMs={}",
                chatId,
                currentBinding.nodeId,
                commit.binding.nodeId,
                commit.binding.generation,
                currentTimeMillis() - routeChangeStartedAt
            ),
        );

        self.installRouteBindingOnTarget(&targetNodeId, commit.operation.clone())
            .await?;

        operit_util::AppLogger::AppLogger::trace(
            "CoreRouteTrace",
            &format!(
                "route_change.lifecycle.after.start chatId={} target={} generation={} elapsedMs={}",
                chatId,
                targetNodeId,
                commit.binding.generation,
                currentTimeMillis() - routeChangeStartedAt
            ),
        );
        let lifecycleResult = self
            .callChatCoreLifecycle(
                &targetNodeId,
                GeneratedRouteLifecycle::AfterChangeRoute,
                chatId,
                Some(resumeContext),
            )
            .await;
        operit_util::AppLogger::AppLogger::trace(
            "CoreRouteTrace",
            &format!(
                "route_change.lifecycle.after.done target={} generation={} elapsedMs={} result={}",
                targetNodeId,
                commit.binding.generation,
                currentTimeMillis() - routeChangeStartedAt,
                if lifecycleResult.is_ok() {
                    "ok"
                } else {
                    "error"
                }
            ),
        );
        lifecycleResult
    }

    /// Installs one committed Binding operation on the target without advancing sync clocks.
    #[allow(non_snake_case)]
    pub(super) async fn installRouteBindingOnTarget(
        &self,
        targetNodeId: &str,
        operation: operit_store::SyncOperationStore::SyncOperation,
    ) -> Result<(), String> {
        if targetNodeId == self.nodeRouter.localNodeId() {
            return Ok(());
        }
        let request = PeerSyncMethod::SyncApplyImmediateBindingOperation.request(
            operit_link::nextCoreRouteRequestId("core-route-binding-install"),
            toCoreValue(serde_json::json!({ "operation": operation }))
                .map_err(|error| error.to_string())?,
        );
        let response = self
            .nodeRouter
            .callNode(targetNodeId.to_string(), request)
            .await;
        let value = response.result.map_err(|error| error.to_string())?;
        let _: serde_json::Value = fromCoreValue(value).map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Invokes one route lifecycle callback on an explicit CoreNode target.
    #[allow(non_snake_case)]
    async fn callChatCoreLifecycle(
        &self,
        targetNodeId: &str,
        lifecycle: GeneratedRouteLifecycle,
        chatId: String,
        resumeContext: Option<CoreRouteResumeContext>,
    ) -> Result<(), String> {
        let route = crate::generated_space_lifecycle_route(lifecycle)
            .ok_or_else(|| format!("route lifecycle hook is not registered: {lifecycle:?}"))?;
        let mut args = BTreeMap::new();
        args.insert(route.bindingArgument.to_string(), CoreValue::String(chatId));
        if let Some(resumeContext) = resumeContext {
            args.insert(
                "resumeContext".to_string(),
                toCoreValue(resumeContext).map_err(|error| error.to_string())?,
            );
        }
        let request = CoreCallRequest::new(
            format!("core-route-lifecycle-{}", currentTimeMillis()),
            CORE_INTERNAL_TARGET,
            route.methodName,
            CoreValue::Map(args),
        );
        let response = if targetNodeId == self.nodeRouter.localNodeId() {
            self.localRuntime.callSpace(request).await
        } else {
            self.nodeRouter
                .callNodeSpace(targetNodeId.to_string(), request)
                .await
        };
        let value = response.result.map_err(|error| error.to_string())?;
        fromCoreValue::<()>(value).map_err(|error| error.to_string())
    }

    /// 配对状态来自统一节点服务，不再发送旧 sessionInfo 请求。
    pub async fn pairedDeviceStatus(
        &self,
        deviceId: String,
    ) -> Result<RuntimePairedDeviceStatus, String> {
        if !self.pairedDevicesSnapshot()?.contains_key(&deviceId) {
            return Ok(RuntimePairedDeviceStatus::Invalid);
        }
        if !self.spaceStore.contains(deviceId.clone())? {
            return Ok(RuntimePairedDeviceStatus::RemovedFromSpace);
        }
        Ok(if self.pairedDeviceOnline(deviceId)? {
            RuntimePairedDeviceStatus::Online
        } else {
            RuntimePairedDeviceStatus::Offline
        })
    }

    /// Disconnects one directly adjacent device while preserving pairing records.
    #[allow(non_snake_case)]
    pub async fn disconnectDeviceSpaceConnection(&self, deviceId: String) -> Result<(), String> {
        let localDeviceId = self.nodeRouter.localNodeId();
        let space = self.spaceStore.initialize()?;
        if !space.members.iter().any(|member| member == &deviceId) {
            return Err(format!(
                "device is not a member of the current device space: {deviceId}"
            ));
        }
        if deviceId == localDeviceId {
            return Err("current device cannot disconnect itself".to_string());
        }
        self.nodeServices()?.peers().disconnectPeer(&deviceId).await
            .map_err(|error| error.to_string())
    }

    /// 按设备撤销两个方向及全部渠道；不再自行编辑旧 session 文件。
    pub async fn removePairedDevice(&self, deviceId: String) -> Result<(), String> {
        self.nodeServices()?.peers().removePairedPeer(&deviceId).await
            .map_err(|error| error.to_string())
    }

    pub fn startSpaceSync(&self) -> Result<(), String> {
        self.persistenceSyncService().start()
    }
    pub fn stopSpaceSync(&self) -> Result<(), String> {
        self.persistenceSyncService().stop()
    }

    /// Builds the persistent synchronization service owned by this runtime facade.
    #[allow(non_snake_case)]
    fn persistenceSyncService(&self) -> SpacePersistenceSyncService {
        self.persistenceSync.clone()
    }
}

/// Converts one synchronized Store profile into the runtime-facing device model.
#[allow(non_snake_case)]
fn runtimeDeviceSpaceDevice(
    profile: &CoreSpaceDeviceProfile,
    online: bool,
    currentIdentity: Option<RuntimeDeviceSpaceIdentity>,
) -> RuntimeDeviceSpaceDevice {
    RuntimeDeviceSpaceDevice {
        deviceId: profile.nodeId.clone(),
        userName: profile.userName.clone(),
        deviceName: profile.displayName.clone(),
        platform: profile.platform.clone(),
        model: profile.model.clone(),
        coreVersion: profile.coreVersion.clone(),
        online,
        currentIdentity,
    }
}

/// Projects the one active identity assigned to a device into the runtime topology.
#[allow(non_snake_case)]
fn runtimeDeviceSpaceIdentity(
    state: &NetworkControlState,
    nodeId: &str,
) -> Option<RuntimeDeviceSpaceIdentity> {
    let identityId = state.deviceIdentityIds.get(nodeId)?;
    let role = state.roles.get(identityId)?;
    let mut capabilities = role.capabilities.iter().cloned().collect::<Vec<_>>();
    capabilities.sort();
    Some(RuntimeDeviceSpaceIdentity {
        displayName: role.displayName.clone(),
        capabilities,
    })
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
        reasons
            .push("Direct Peer Link status is not observable from the current device".to_string());
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

/// Maps paired devices to online states using Peer Links.
#[allow(non_snake_case)]
fn pairedDeviceStatusesFromState(
    pairedDevices: BTreeMap<String, RuntimePairedDevice>,
    activePeerNodeIds: BTreeSet<String>,
) -> BTreeMap<String, RuntimePairedDeviceStatus> {
    pairedDevices
        .into_keys()
        .map(|deviceId| {
            let status = if activePeerNodeIds.contains(&deviceId) {
                RuntimePairedDeviceStatus::Online
            } else {
                RuntimePairedDeviceStatus::Offline
            };
            (deviceId, status)
        })
        .collect()
}

/// 只允许当前 Space 加上已鉴权的一个节点；保留原有身份、成员与版本校验。
fn validateSpaceJoin(current: &CoreSpace, peerNodeId: &str, proposal: &CoreSpace) -> Result<(), String> {
    let mut expected = current.members.iter().cloned().collect::<BTreeSet<_>>();
    let isNewMember = expected.insert(peerNodeId.to_string());
    let proposed = proposal.members.iter().cloned().collect::<BTreeSet<_>>();
    if proposed != expected || proposed.len() != proposal.members.len() {
        return Err("join proposal members must equal the current Space plus the authenticated device".into());
    }
    if proposal.spaceId != current.spaceId || proposal.spaceName != current.spaceName {
        return Err("join proposal must preserve the server Space identity".into());
    }
    let revision = if isNewMember {
        current.spaceRevision.checked_add(1).ok_or("Space revision overflow during join")?
    } else { current.spaceRevision };
    if proposal.spaceRevision != revision {
        return Err("join proposal has an invalid Space revision".into());
    }
    Ok(())
}

/// Uses map's existing weak target and automatic upstream unsubscription.
/// The map closure owns the stop sender; removing its last subscriber drops
/// the sender even while the worker still owns and updates the source state.
fn spaceOverviewSubscription<T>(source: &StateFlow<T>, stop: oneshot::Sender<()>) -> StateFlow<T>
where
    T: Clone + PartialEq + Send + 'static,
{
    source.map(move |snapshot| {
        let _keepWorkerAlive = &stop;
        snapshot
    })
}

#[cfg(test)]
mod device_space_facade_tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/device_space/facade_state.rs"));
}
