use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use crate::CoreSpaceStore::{CoreSpaceDeviceProfile, CoreSpaceStore};
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;
use crate::SyncOperationStore::{
    NewSyncOperation, SyncClock, SyncOperation, SyncOperationOrder, SyncOperationSemantics,
    SyncOperationStore,
};

/// Names the synchronized domain that owns Space authorization and control commands.
pub const NETWORK_CONTROL_SYNC_DOMAIN: &str = "network_control";
const NETWORK_CONTROL_ENTITY_TYPE: &str = "command";
const NETWORK_CONTROL_OPERATION: &str = "apply";

static NETWORK_CONTROL_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Describes one role whose identifier and capabilities are owned by a Space policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlRole {
    pub roleId: String,
    pub displayName: String,
    pub capabilities: BTreeSet<String>,
}

/// Names the capabilities that control identity administration and approval.
pub const NETWORK_IDENTITY_MANAGE_CAPABILITY: &str = "network.identity.manage";
pub const NETWORK_IDENTITY_ASSIGN_CAPABILITY: &str = "network.identity.assign";
pub const NETWORK_APPROVAL_CAPABILITY: &str = "network.approval";
pub const NETWORK_DEVICES_VIEW_CAPABILITY: &str = "network.devices.view";

/// Carries one device identity assignment through the synchronized command log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlIdentityAssignment {
    pub nodeId: String,
    pub roleId: String,
}

/// Carries the mutable policy commands replicated through the Space operation log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkControlCommand {
    Bootstrap {
        initialAdminNodeId: String,
    },
    DefineRole {
        role: NetworkControlRole,
    },
    GrantRole {
        grant: NetworkControlIdentityAssignment,
    },
    RevokeRole {
        nodeId: String,
    },
    AdmitMember {
        nodeId: String,
    },
    RemoveMember {
        nodeId: String,
    },
    DisconnectNode {
        nodeId: String,
    },
    PolicyUpdate {
        policyId: String,
        value: String,
    },
}

/// Records one issuer-bound command before it is encoded as a synchronization operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlCommandRecord {
    pub commandId: String,
    pub spaceId: String,
    pub issuerNodeId: String,
    pub command: NetworkControlCommand,
}

/// Reports the local authorization decision for one replicated control command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlAuditRecord {
    pub commandId: String,
    pub spaceId: String,
    pub issuerNodeId: String,
    pub originNodeId: String,
    pub accepted: bool,
    pub reason: String,
    pub summary: String,
    pub recordedAt: i64,
}

/// Materializes the authorization state selected by accepted control commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkControlState {
    pub spaceId: String,
    pub initialized: bool,
    pub memberNodeIds: BTreeSet<String>,
    pub roles: BTreeMap<String, NetworkControlRole>,
    pub deviceIdentityIds: BTreeMap<String, String>,
    pub removedNodeIds: BTreeSet<String>,
    pub disconnectedNodeIds: BTreeSet<String>,
    pub policies: BTreeMap<String, String>,
}

/// Owns the compact, auditable authorization policy for one synchronized Space.
#[derive(Clone)]
pub struct NetworkControlStore {
    spaceStore: CoreSpaceStore,
    syncOperationStore: SyncOperationStore,
    localNodeId: String,
}

impl NetworkControlStore {
    /// Opens the Space control store over one runtime storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Result<Self, String> {
        let localNodeId = CoreNodeIdentityStore::new(storage.clone())
            .initialize()?
            .nodeId;
        Ok(Self {
            spaceStore: CoreSpaceStore::new(storage.clone()),
            syncOperationStore: SyncOperationStore::new(storage.clone(), RUNTIME_SYNC_DIR_PATH),
            localNodeId,
        })
    }

    /// Opens the Space control store over the process-wide runtime storage host.
    pub fn native() -> Result<Self, String> {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Creates the initial administrator command for a newly created single-device Space.
    #[allow(non_snake_case)]
    pub fn bootstrapCurrentSpace(&self) -> Result<NetworkControlState, String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let space = self.spaceStore.initialize()?;
        if space.members.len() != 1 || space.members.first() != Some(&self.localNodeId) {
            return Err(
                "only a new single-device Space can bootstrap its control policy".to_string(),
            );
        }
        let state = self.materializeState(&space.spaceId)?;
        if state.initialized {
            return Err("Space control policy is already initialized".to_string());
        }
        self.appendLocalCommand(
            &space.spaceId,
            NetworkControlCommand::Bootstrap {
                initialAdminNodeId: self.localNodeId.clone(),
            },
        )?;
        self.materializeState(&space.spaceId)
    }

    /// Returns the initialized Space control policy, bootstrapping only the creator's new Space.
    #[allow(non_snake_case)]
    pub fn initializeCurrentSpace(&self) -> Result<NetworkControlState, String> {
        let state = self.currentState()?;
        if state.initialized {
            return Ok(state);
        }
        let space = self.spaceStore.initialize()?;
        if space.members.len() == 1 && space.members.first() == Some(&self.localNodeId) {
            return self.bootstrapCurrentSpace();
        }
        Ok(state)
    }

    /// Returns the accepted authorization state for the current Space.
    #[allow(non_snake_case)]
    pub fn currentState(&self) -> Result<NetworkControlState, String> {
        let space = self.spaceStore.initialize()?;
        self.materializeState(&space.spaceId)
    }

    /// Returns the complete accepted and rejected command audit for the current Space.
    pub fn audit(&self) -> Result<Vec<NetworkControlAuditRecord>, String> {
        let space = self.spaceStore.initialize()?;
        self.materialize(&space.spaceId).map(|(_, audit)| audit)
    }

    /// Returns whether one node has one capability for an optional managed target.
    #[allow(non_snake_case)]
    pub fn nodeHasCapability(
        &self,
        nodeId: &str,
        capability: &str,
        targetNodeId: Option<&str>,
    ) -> Result<bool, String> {
        validateNodeId(nodeId)?;
        validateCapability(capability)?;
        if let Some(targetNodeId) = targetNodeId {
            validateNodeId(targetNodeId)?;
        }
        let state = self.currentState()?;
        Ok(hasCapability(&state, nodeId, capability, targetNodeId))
    }

    /// Returns whether one node has been removed from the current Space policy.
    #[allow(non_snake_case)]
    pub fn nodeIsRemoved(&self, nodeId: &str) -> Result<bool, String> {
        validateNodeId(nodeId)?;
        Ok(self.currentState()?.removedNodeIds.contains(nodeId))
    }

    /// Returns whether a node is prohibited from being used as a direct connection or route hop.
    #[allow(non_snake_case)]
    pub fn nodeIsDisconnected(&self, nodeId: &str) -> Result<bool, String> {
        validateNodeId(nodeId)?;
        let state = self.currentState()?;
        Ok(state.removedNodeIds.contains(nodeId) || state.disconnectedNodeIds.contains(nodeId))
    }

    /// Returns the current Space members explicitly permitted to carry a routed later hop.
    #[allow(non_snake_case)]
    pub fn relayNodeIds(&self) -> Result<BTreeSet<String>, String> {
        let space = self.spaceStore.initialize()?;
        let state = self.materializeState(&space.spaceId)?;
        Ok(space
            .members
            .into_iter()
            .filter(|nodeId| hasCapability(&state, nodeId, "network.relay", None))
            .collect())
    }

    /// Defines one custom role after checking that the issuer can define every requested ability.
    #[allow(non_snake_case)]
    pub fn defineRole(&self, role: NetworkControlRole) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::DefineRole { role })
    }

    /// Sets one existing identity on a device after checking capability limits.
    #[allow(non_snake_case)]
    pub fn setIdentity(
        &self,
        assignment: NetworkControlIdentityAssignment,
    ) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&assignment.nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::GrantRole { grant: assignment })
    }

    /// Clears the current identity from one device.
    #[allow(non_snake_case)]
    pub fn clearIdentity(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::RevokeRole { nodeId })
    }

    /// Removes a member from the current Space authorization policy.
    #[allow(non_snake_case)]
    pub fn removeMember(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::RemoveMember { nodeId })
    }

    /// Admits one authenticated device as a member of the current Space.
    #[allow(non_snake_case)]
    pub fn admitMember(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::AdmitMember { nodeId })
    }

    /// Prevents a device from being used as a direct connection or transit route.
    #[allow(non_snake_case)]
    pub fn disconnectNode(&self, nodeId: String) -> Result<SyncOperation, String> {
        self.requireKnownSpaceMember(&nodeId)?;
        self.submitLocalCommand(NetworkControlCommand::DisconnectNode { nodeId })
    }

    /// Updates one policy value after the issuer proves policy-management capability.
    #[allow(non_snake_case)]
    pub fn updatePolicy(&self, policyId: String, value: String) -> Result<SyncOperation, String> {
        self.submitLocalCommand(NetworkControlCommand::PolicyUpdate { policyId, value })
    }

    /// Applies one received control command without emitting a second local command.
    #[allow(non_snake_case)]
    pub fn applySyncedOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        validateControlOperation(operation)?;
        self.syncOperationStore
            .appendOperation(operation)
            .map_err(|error| error.to_string())
    }

    /// Applies a bootstrap control command without local conflict filtering.
    #[allow(non_snake_case)]
    pub fn applyBootstrapOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        self.applySyncedOperation(operation)
    }

    /// Submits a local command after materializing the latest Space policy under one lock.
    #[allow(non_snake_case)]
    fn submitLocalCommand(&self, command: NetworkControlCommand) -> Result<SyncOperation, String> {
        let _lock = networkControlMutationLock()
            .lock()
            .map_err(|error| format!("Network control mutation lock poisoned: {error}"))?;
        let space = self.spaceStore.initialize()?;
        let state = self.materializeState(&space.spaceId)?;
        authorizeCommand(&state, &self.localNodeId, &command)?;
        self.appendLocalCommand(&space.spaceId, command)
    }

    /// Rejects a policy mutation whose target device does not belong to this Space.
    #[allow(non_snake_case)]
    fn requireKnownSpaceMember(&self, nodeId: &str) -> Result<(), String> {
        validateNodeId(nodeId)?;
        if self.currentState()?.memberNodeIds.contains(nodeId) {
            return Ok(());
        }
        Err(format!(
            "device has not been admitted to the current Space: {nodeId}"
        ))
    }

    /// Appends one issuer-bound command into the common synchronization operation log.
    #[allow(non_snake_case)]
    fn appendLocalCommand(
        &self,
        spaceId: &str,
        command: NetworkControlCommand,
    ) -> Result<SyncOperation, String> {
        let record = NetworkControlCommandRecord {
            commandId: format!("control-{}", Uuid::new_v4().simple()),
            spaceId: spaceId.to_string(),
            issuerNodeId: self.localNodeId.clone(),
            command,
        };
        self.syncOperationStore
            .appendLocalOperation(
                &self.localNodeId,
                NewSyncOperation {
                    domain: NETWORK_CONTROL_SYNC_DOMAIN.to_string(),
                    entityType: NETWORK_CONTROL_ENTITY_TYPE.to_string(),
                    entityId: record.commandId.clone(),
                    operation: NETWORK_CONTROL_OPERATION.to_string(),
                    semantics: SyncOperationSemantics::Transaction,
                    payload: serde_json::to_value(record).map_err(|error| error.to_string())?,
                },
            )
            .map_err(|error| error.to_string())
    }

    /// Materializes only the accepted state for one current Space identity.
    #[allow(non_snake_case)]
    fn materializeState(&self, spaceId: &str) -> Result<NetworkControlState, String> {
        self.materialize(spaceId).map(|(state, _)| state)
    }

    /// Replays the current Space command log in deterministic order and emits its audit decisions.
    fn materialize(
        &self,
        spaceId: &str,
    ) -> Result<(NetworkControlState, Vec<NetworkControlAuditRecord>), String> {
        validateSpaceId(spaceId)?;
        let mut commands = self
            .syncOperationStore
            .operationsSince(
                &SyncClock::empty(),
                &[NETWORK_CONTROL_SYNC_DOMAIN.to_string()],
                usize::MAX,
            )
            .map_err(|error| error.to_string())?;
        commands
            .sort_by(|left, right| controlOperationOrder(left).cmp(&controlOperationOrder(right)));
        let mut state = NetworkControlState {
            spaceId: spaceId.to_string(),
            initialized: false,
            memberNodeIds: BTreeSet::new(),
            roles: builtinRoles(),
            deviceIdentityIds: BTreeMap::new(),
            removedNodeIds: BTreeSet::new(),
            disconnectedNodeIds: BTreeSet::new(),
            policies: BTreeMap::new(),
        };
        let mut audit = Vec::new();
        let profiles = self.spaceStore.deviceProfiles()?;
        for operation in commands {
            let record = decodeControlOperation(&operation)?;
            if record.spaceId != spaceId {
                continue;
            }
            let summary = controlAuditSummary(&record.command, &state, &profiles)?;
            let result = authorizeAndApplyCommand(&mut state, &record, &operation.originDeviceId);
            let (accepted, reason) = match result {
                Ok(()) => (true, "accepted".to_string()),
                Err(reason) => (false, reason),
            };
            audit.push(NetworkControlAuditRecord {
                commandId: record.commandId,
                spaceId: record.spaceId,
                issuerNodeId: record.issuerNodeId,
                originNodeId: operation.originDeviceId,
                accepted,
                reason,
                summary,
                recordedAt: operation.createdAt,
            });
        }
        Ok((state, audit))
    }
}

/// Returns the process-wide mutex that serializes local policy decisions and writes.
#[allow(non_snake_case)]
fn networkControlMutationLock() -> &'static Mutex<()> {
    NETWORK_CONTROL_MUTATION_LOCK.get_or_init(|| Mutex::new(()))
}

/// Returns the immutable built-in roles exposed alongside user-defined roles.
#[allow(non_snake_case)]
fn builtinRoles() -> BTreeMap<String, NetworkControlRole> {
    [
        builtinRole("admin", "Administrator", ["*"]),
        builtinRole(
            "user",
            "User",
            [NETWORK_DEVICES_VIEW_CAPABILITY, "network.user"],
        ),
        builtinRole("relay", "Relay", ["network.relay"]),
        builtinRole("storage", "Storage", ["storage.provide"]),
        builtinRole("runner", "Runner", ["runtime.execute"]),
        builtinRole("auditor", "Auditor", ["network.audit.read"]),
    ]
    .into_iter()
    .map(|role| (role.roleId.clone(), role))
    .collect()
}

/// Creates one fixed built-in role definition.
#[allow(non_snake_case)]
fn builtinRole(
    roleId: &str,
    displayName: &str,
    capabilities: impl IntoIterator<Item = &'static str>,
) -> NetworkControlRole {
    NetworkControlRole {
        roleId: roleId.to_string(),
        displayName: displayName.to_string(),
        capabilities: capabilities.into_iter().map(str::to_string).collect(),
    }
}

/// Builds a concise human-readable audit description for the management UI.
fn controlAuditSummary(
    command: &NetworkControlCommand,
    state: &NetworkControlState,
    profiles: &std::collections::BTreeMap<String, CoreSpaceDeviceProfile>,
) -> Result<String, String> {
    let summary = match command {
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => format!(
            "Bootstrap control; administrator {}",
            controlDeviceLabel(profiles, initialAdminNodeId)?
        ),
        NetworkControlCommand::DefineRole { role } => format!("Define role {}", role.displayName),
        NetworkControlCommand::GrantRole { grant: assignment } => format!(
            "Set identity {} on {}",
            controlRoleLabel(state, &assignment.roleId)?,
            controlDeviceLabel(profiles, &assignment.nodeId)?
        ),
        NetworkControlCommand::RevokeRole { nodeId } => {
            format!(
                "Clear identity from {}",
                controlDeviceLabel(profiles, nodeId)?
            )
        }
        NetworkControlCommand::AdmitMember { nodeId } => {
            format!("Admit device {}", controlDeviceLabel(profiles, nodeId)?)
        }
        NetworkControlCommand::RemoveMember { nodeId } => {
            format!("Remove device {}", controlDeviceLabel(profiles, nodeId)?)
        }
        NetworkControlCommand::DisconnectNode { nodeId } => format!(
            "Disconnect device {}",
            controlDeviceLabel(profiles, nodeId)?
        ),
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            format!("Update policy {} to {:?}", policyId, value)
        }
    };
    Ok(summary)
}

/// Formats one device using its human-facing name and owner.
fn controlDeviceLabel(
    profiles: &std::collections::BTreeMap<String, CoreSpaceDeviceProfile>,
    nodeId: &str,
) -> Result<String, String> {
    let profile = profiles
        .get(nodeId)
        .ok_or_else(|| format!("device profile is missing for audit device: {nodeId}"))?;
    if profile.userName.trim().is_empty() {
        Ok(profile.displayName.clone())
    } else {
        Ok(format!("{} · {}", profile.displayName, profile.userName))
    }
}

/// Formats one role using its human-facing name.
fn controlRoleLabel(state: &NetworkControlState, roleId: &str) -> Result<String, String> {
    let role = state
        .roles
        .get(roleId)
        .ok_or_else(|| format!("role is missing for audit command: {roleId}"))?;
    Ok(role.displayName.clone())
}

/// Returns the deterministic order used to replay Space control commands.
#[allow(non_snake_case)]
fn controlOperationOrder(operation: &SyncOperation) -> SyncOperationOrder {
    SyncOperationOrder::fromOperation(operation)
}

/// Decodes one structurally valid control operation into its command payload.
#[allow(non_snake_case)]
fn decodeControlOperation(
    operation: &SyncOperation,
) -> Result<NetworkControlCommandRecord, String> {
    validateControlOperation(operation)?;
    let record: NetworkControlCommandRecord =
        serde_json::from_value(operation.payload.clone()).map_err(|error| error.to_string())?;
    validateCommandRecord(&record)?;
    Ok(record)
}

/// Validates the immutable synchronization envelope for a Space control command.
#[allow(non_snake_case)]
fn validateControlOperation(operation: &SyncOperation) -> Result<(), String> {
    if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN
        || operation.entityType != NETWORK_CONTROL_ENTITY_TYPE
        || operation.operation != NETWORK_CONTROL_OPERATION
        || operation.semantics != SyncOperationSemantics::Transaction
    {
        return Err("operation is not a network control command".to_string());
    }
    let record: NetworkControlCommandRecord =
        serde_json::from_value(operation.payload.clone()).map_err(|error| error.to_string())?;
    validateCommandRecord(&record)?;
    if record.commandId != operation.entityId {
        return Err("network control command id does not match its operation entity".to_string());
    }
    Ok(())
}

/// Validates the fields carried by one command before it is authorized.
#[allow(non_snake_case)]
fn validateCommandRecord(record: &NetworkControlCommandRecord) -> Result<(), String> {
    validateIdentifier("command id", &record.commandId, 160)?;
    validateSpaceId(&record.spaceId)?;
    validateNodeId(&record.issuerNodeId)?;
    match &record.command {
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => {
            validateNodeId(initialAdminNodeId)
        }
        NetworkControlCommand::DefineRole { role } => validateRole(role),
        NetworkControlCommand::GrantRole { grant: assignment } => {
            validateIdentityAssignment(assignment)
        }
        NetworkControlCommand::RevokeRole { nodeId } => validateNodeId(nodeId),
        NetworkControlCommand::AdmitMember { nodeId }
        | NetworkControlCommand::RemoveMember { nodeId }
        | NetworkControlCommand::DisconnectNode { nodeId } => validateNodeId(nodeId),
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            validateIdentifier("policy id", policyId, 160)?;
            validateIdentifier("policy value", value, 4_096)
        }
    }
}

/// Authorizes one local command without changing the materialized policy state.
#[allow(non_snake_case)]
fn authorizeCommand(
    state: &NetworkControlState,
    issuerNodeId: &str,
    command: &NetworkControlCommand,
) -> Result<(), String> {
    let mut candidate = state.clone();
    let record = NetworkControlCommandRecord {
        commandId: "local-authorization".to_string(),
        spaceId: state.spaceId.clone(),
        issuerNodeId: issuerNodeId.to_string(),
        command: command.clone(),
    };
    authorizeAndApplyCommand(&mut candidate, &record, issuerNodeId)
}

/// Checks issuer provenance and applies a command to the selected policy state when authorized.
#[allow(non_snake_case)]
fn authorizeAndApplyCommand(
    state: &mut NetworkControlState,
    record: &NetworkControlCommandRecord,
    originNodeId: &str,
) -> Result<(), String> {
    if record.issuerNodeId != originNodeId {
        return Err("network control issuer does not match operation origin".to_string());
    }
    if state.removedNodeIds.contains(&record.issuerNodeId) {
        return Err("removed node cannot issue network control commands".to_string());
    }
    match &record.command {
        NetworkControlCommand::Bootstrap { initialAdminNodeId } => {
            if state.initialized {
                return Err("Space control policy already has an initial administrator".to_string());
            }
            if initialAdminNodeId != &record.issuerNodeId {
                return Err(
                    "initial administrator must issue its own bootstrap command".to_string()
                );
            }
            state.initialized = true;
            state.memberNodeIds.insert(initialAdminNodeId.clone());
            state
                .deviceIdentityIds
                .insert(initialAdminNodeId.clone(), "admin".to_string());
            Ok(())
        }
        NetworkControlCommand::DefineRole { role } => {
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if builtinRoles().contains_key(&role.roleId) {
                return Err("built-in roles cannot be redefined".to_string());
            }
            if state.roles.contains_key(&role.roleId) {
                return Err(format!(
                    "network control role already exists: {}",
                    role.roleId
                ));
            }
            for capability in &role.capabilities {
                requireGlobalCapability(state, &record.issuerNodeId, capability)?;
            }
            state.roles.insert(role.roleId.clone(), role.clone());
            Ok(())
        }
        NetworkControlCommand::GrantRole { grant: assignment } => {
            let role = state.roles.get(&assignment.roleId).ok_or_else(|| {
                format!(
                    "network control identity does not exist: {}",
                    assignment.roleId
                )
            })?;
            if state.removedNodeIds.contains(&assignment.nodeId) {
                return Err("removed device cannot receive an identity".to_string());
            }
            if !state.memberNodeIds.contains(&assignment.nodeId) {
                return Err("identity target has not been admitted to the Space".to_string());
            }
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_ASSIGN_CAPABILITY,
            )?;
            for capability in &role.capabilities {
                requireGlobalCapability(state, &record.issuerNodeId, capability)?;
            }
            if let Some(existingIdentityId) = state.deviceIdentityIds.get(&assignment.nodeId) {
                if assignment.roleId != "admin"
                    && existingIdentityId == "admin"
                    && clearingIdentityLeavesNoAdministrator(state, &assignment.nodeId)
                {
                    return Err("Space control policy must retain an administrator".to_string());
                }
            }
            state
                .deviceIdentityIds
                .insert(assignment.nodeId.clone(), assignment.roleId.clone());
            Ok(())
        }
        NetworkControlCommand::RevokeRole { nodeId } => {
            let identityId = state
                .deviceIdentityIds
                .get(nodeId)
                .ok_or_else(|| format!("network device has no identity: {nodeId}"))?;
            requireGlobalCapability(
                state,
                &record.issuerNodeId,
                NETWORK_IDENTITY_MANAGE_CAPABILITY,
            )?;
            if identityId == "admin" && clearingIdentityLeavesNoAdministrator(state, nodeId) {
                return Err("Space control policy must retain an administrator".to_string());
            }
            state.deviceIdentityIds.remove(nodeId);
            Ok(())
        }
        NetworkControlCommand::AdmitMember { nodeId } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.members.join",
                Some(nodeId),
            )?;
            state.removedNodeIds.remove(nodeId);
            state.disconnectedNodeIds.remove(nodeId);
            state.memberNodeIds.insert(nodeId.clone());
            if !state.deviceIdentityIds.contains_key(nodeId) {
                state
                    .deviceIdentityIds
                    .insert(nodeId.clone(), "user".to_string());
            }
            Ok(())
        }
        NetworkControlCommand::RemoveMember { nodeId } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.members.remove",
                Some(nodeId),
            )?;
            if nodeIsSoleAdministrator(state, nodeId) {
                return Err("Space control policy must retain an administrator".to_string());
            }
            state.deviceIdentityIds.remove(nodeId);
            state.memberNodeIds.insert(nodeId.clone());
            state.removedNodeIds.insert(nodeId.clone());
            state.disconnectedNodeIds.insert(nodeId.clone());
            Ok(())
        }
        NetworkControlCommand::DisconnectNode { nodeId } => {
            requireCapability(
                state,
                &record.issuerNodeId,
                "network.connections.disconnect",
                Some(nodeId),
            )?;
            if !state.memberNodeIds.contains(nodeId) {
                return Err("device has not been admitted to the Space".to_string());
            }
            state.disconnectedNodeIds.insert(nodeId.clone());
            Ok(())
        }
        NetworkControlCommand::PolicyUpdate { policyId, value } => {
            requireGlobalCapability(state, &record.issuerNodeId, "network.policy.update")?;
            state.policies.insert(policyId.clone(), value.clone());
            Ok(())
        }
    }
}

/// Requires a capability independent of a particular managed device.
#[allow(non_snake_case)]
fn requireGlobalCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
) -> Result<(), String> {
    requireCapability(state, nodeId, capability, None)
}

/// Requires one capability for an optional target device.
#[allow(non_snake_case)]
fn requireCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
    targetNodeId: Option<&str>,
) -> Result<(), String> {
    if hasCapability(state, nodeId, capability, targetNodeId) {
        return Ok(());
    }
    Err(format!("node {nodeId} lacks capability {capability}"))
}

/// Resolves one capability from the device's current identity.
#[allow(non_snake_case)]
fn hasCapability(
    state: &NetworkControlState,
    nodeId: &str,
    capability: &str,
    targetNodeId: Option<&str>,
) -> bool {
    if !state.initialized || state.removedNodeIds.contains(nodeId) {
        return false;
    }
    let Some(identityId) = state.deviceIdentityIds.get(nodeId) else {
        return false;
    };
    let Some(role) = state.roles.get(identityId) else {
        return false;
    };
    let _ = targetNodeId;
    role.capabilities.contains("*") || role.capabilities.contains(capability)
}

/// Reports whether clearing one device identity would leave the Space without an administrator.
#[allow(non_snake_case)]
fn clearingIdentityLeavesNoAdministrator(state: &NetworkControlState, nodeId: &str) -> bool {
    !state
        .deviceIdentityIds
        .iter()
        .any(|(existingNodeId, identityId)| {
            existingNodeId != nodeId
                && identityId == "admin"
                && !state.removedNodeIds.contains(existingNodeId)
        })
}

/// Reports whether one device currently owns the only active administrator identity.
#[allow(non_snake_case)]
fn nodeIsSoleAdministrator(state: &NetworkControlState, nodeId: &str) -> bool {
    matches!(state.deviceIdentityIds.get(nodeId), Some(identityId) if identityId == "admin")
        && !state.removedNodeIds.contains(nodeId)
        && !state
            .deviceIdentityIds
            .iter()
            .any(|(existingNodeId, identityId)| {
                existingNodeId != nodeId
                    && identityId == "admin"
                    && !state.removedNodeIds.contains(existingNodeId)
            })
}

/// Validates one role definition before it is replicated through the control log.
#[allow(non_snake_case)]
fn validateRole(role: &NetworkControlRole) -> Result<(), String> {
    validateIdentifier("role id", &role.roleId, 120)?;
    validateIdentifier("role display name", &role.displayName, 160)?;
    if role.capabilities.is_empty() {
        return Err("network control role must declare at least one capability".to_string());
    }
    for capability in &role.capabilities {
        validateCapability(capability)?;
    }
    Ok(())
}

/// Validates one identity assignment before it is replicated through the control log.
#[allow(non_snake_case)]
fn validateIdentityAssignment(assignment: &NetworkControlIdentityAssignment) -> Result<(), String> {
    validateNodeId(&assignment.nodeId)?;
    validateIdentifier("role id", &assignment.roleId, 120)
}

/// Validates one Space identity carried by a control command.
#[allow(non_snake_case)]
fn validateSpaceId(spaceId: &str) -> Result<(), String> {
    validateIdentifier("space id", spaceId, 160)
}

/// Validates one CoreNode identifier carried by a control command.
#[allow(non_snake_case)]
fn validateNodeId(nodeId: &str) -> Result<(), String> {
    validateIdentifier("node id", nodeId, 160)
}

/// Validates one named capability without parsing or inferring capability syntax.
#[allow(non_snake_case)]
fn validateCapability(capability: &str) -> Result<(), String> {
    validateIdentifier("capability", capability, 160)
}

/// Validates a nonempty bounded text identifier used by the control policy.
#[allow(non_snake_case)]
fn validateIdentifier(fieldName: &str, value: &str, maximumLength: usize) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("network control {fieldName} must not be empty"));
    }
    if trimmed.chars().count() > maximumLength {
        return Err(format!(
            "network control {fieldName} exceeds {maximumLength} characters"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!(
            "network control {fieldName} must not contain control characters"
        ));
    }
    Ok(())
}
