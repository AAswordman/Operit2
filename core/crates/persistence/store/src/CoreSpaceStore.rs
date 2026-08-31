use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_util::RuntimeStorageLayout::{
    RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH, RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH,
    RUNTIME_SPACE_MEMBERS_DIR_PATH, RUNTIME_SPACE_TOPOLOGY_DIR_PATH,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use crate::PreferencesDataStore::{emptyPreferences, stringPreferencesKey, PreferencesDataStore};
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;

const CORE_SPACE_RECORD_KEY: &str = "record";

/// Describes the converged Space membership visible to one CoreNode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSpace {
    pub spaceId: String,
    pub spaceName: String,
    pub spaceRevision: i64,
    pub members: Vec<String>,
}

/// Describes one synchronized device presentation inside a device space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSpaceDeviceProfile {
    pub nodeId: String,
    pub displayName: String,
    pub userName: String,
    pub platform: String,
    pub model: String,
    #[serde(default)]
    pub coreVersion: Option<String>,
    pub updatedAt: i64,
}

/// Describes one directed direct-device connection inside a device space.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoreSpaceDeviceConnection {
    pub firstDeviceId: String,
    pub secondDeviceId: String,
}

/// Describes one synchronized device availability announcement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSpaceDevicePresence {
    pub nodeId: String,
    pub active: bool,
    pub baseUrl: String,
    pub tokenHash: String,
    pub version: String,
    pub updatedAt: i64,
}

/// Stores one independently synchronized Space member record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CoreSpaceMemberRecord {
    spaceId: String,
    spaceName: String,
    spaceRevision: i64,
    nodeId: String,
    joinedAt: i64,
    updatedAt: i64,
}

/// Stores the directly paired peers announced by one CoreNode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CoreSpaceTopologyRecord {
    nodeId: String,
    peers: Vec<String>,
    updatedAt: i64,
}

/// Persists Space membership as one synchronized preferences entity per CoreNode.
#[derive(Clone)]
pub struct CoreSpaceStore {
    storage: Arc<dyn RuntimeStorageHost>,
}

impl CoreSpaceStore {
    /// Creates a Space store over an explicit runtime storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }

    /// Creates a Space store over the process-wide runtime storage host.
    pub fn native() -> Self {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Creates the local singleton Space membership and returns the converged state.
    pub fn initialize(&self) -> Result<CoreSpace, String> {
        self.initializeNamed(defaultSpaceName())
    }

    /// Creates the local singleton Space with an explicit initial display name.
    #[allow(non_snake_case)]
    pub fn initializeNamed(&self, initialSpaceName: String) -> Result<CoreSpace, String> {
        validateSpaceName(&initialSpaceName)?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let records = self.memberRecords()?;
        if records.contains_key(&identity.nodeId) {
            return coreSpaceFromRecords(records, &identity.nodeId);
        }
        let now = currentTimeMillis();
        let record = CoreSpaceMemberRecord {
            spaceId: newSpaceId(),
            spaceName: initialSpaceName,
            spaceRevision: 1,
            nodeId: identity.nodeId,
            joinedAt: now,
            updatedAt: now,
        };
        self.writeMemberRecord(&record)?;
        coreSpaceFromRecords(self.memberRecords()?, &record.nodeId)
    }

    /// Returns the current converged Space membership.
    pub fn space(&self) -> Result<CoreSpace, String> {
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        coreSpaceFromRecords(self.memberRecords()?, &identity.nodeId)
    }

    /// Joins an explicitly selected peer Space while preserving the peer identity.
    pub fn merge(&self, peerSpace: CoreSpace) -> Result<CoreSpace, String> {
        validateCoreSpace(&peerSpace)?;
        let localSpace = self.initialize()?;
        let members = localSpace
            .members
            .into_iter()
            .chain(peerSpace.members)
            .collect::<BTreeSet<_>>();
        let nextRevision = nextSpaceRevision(localSpace.spaceRevision, peerSpace.spaceRevision)?;
        self.writeSpaceProjection(
            peerSpace.spaceId,
            peerSpace.spaceName,
            nextRevision,
            members,
        )
    }

    /// Adopts a joined Space projection produced by an explicit pairing workflow.
    pub fn adopt(&self, joinedSpace: CoreSpace) -> Result<CoreSpace, String> {
        validateCoreSpace(&joinedSpace)?;
        let localSpace = self.initialize()?;
        if joinedSpace.spaceRevision < localSpace.spaceRevision {
            return Err(format!(
                "joined device space revision is older: local={}, incoming={}",
                localSpace.spaceRevision, joinedSpace.spaceRevision
            ));
        }
        if joinedSpace.spaceRevision == localSpace.spaceRevision
            && (joinedSpace.spaceId != localSpace.spaceId
                || joinedSpace.spaceName != localSpace.spaceName)
        {
            return Err("joined device space identity conflicts at the same revision".to_string());
        }
        let members = localSpace
            .members
            .into_iter()
            .chain(joinedSpace.members)
            .collect::<BTreeSet<_>>();
        self.writeSpaceProjection(
            joinedSpace.spaceId,
            joinedSpace.spaceName,
            joinedSpace.spaceRevision,
            members,
        )
    }

    /// Records the Space projection announced by one directly paired device.
    #[allow(non_snake_case)]
    pub fn observePairedDeviceSpace(
        &self,
        pairedNodeId: String,
        pairedSpace: CoreSpace,
    ) -> Result<CoreSpace, String> {
        validateNodeId(&pairedNodeId)?;
        validateCoreSpace(&pairedSpace)?;
        if !pairedSpace
            .members
            .iter()
            .any(|member| member == &pairedNodeId)
        {
            return Err("Paired device is not a member of its announced device space".to_string());
        }

        let localSpace = self.initialize()?;
        let records = self.memberRecords()?;
        if let Some(currentPeerRecord) = records.get(&pairedNodeId) {
            if pairedSpace.spaceRevision < currentPeerRecord.spaceRevision {
                return Ok(localSpace);
            }
            if pairedSpace.spaceRevision == currentPeerRecord.spaceRevision
                && (pairedSpace.spaceId != currentPeerRecord.spaceId
                    || pairedSpace.spaceName != currentPeerRecord.spaceName)
            {
                return Err(
                    "Paired device space identity conflicts at the same revision".to_string(),
                );
            }
        }

        if pairedSpace.spaceId == localSpace.spaceId {
            if pairedSpace.spaceRevision < localSpace.spaceRevision {
                return Ok(localSpace);
            }
            if pairedSpace.spaceRevision == localSpace.spaceRevision
                && pairedSpace.spaceName != localSpace.spaceName
            {
                return Err("Device space name conflicts at the same revision".to_string());
            }
            let members = localSpace
                .members
                .into_iter()
                .chain(pairedSpace.members)
                .collect::<BTreeSet<_>>();
            return self.writeSpaceProjection(
                pairedSpace.spaceId,
                pairedSpace.spaceName,
                pairedSpace.spaceRevision,
                members,
            );
        }

        let now = currentTimeMillis();
        let record = CoreSpaceMemberRecord {
            spaceId: pairedSpace.spaceId,
            spaceName: pairedSpace.spaceName,
            spaceRevision: pairedSpace.spaceRevision,
            nodeId: pairedNodeId.clone(),
            joinedAt: records
                .get(&pairedNodeId)
                .map(|record| record.joinedAt)
                .unwrap_or(now),
            updatedAt: now,
        };
        if !records
            .get(&pairedNodeId)
            .map(|existing| existing.hasSameMembershipAs(&record))
            .unwrap_or(false)
        {
            self.writeMemberRecord(&record)?;
        }
        self.space()
    }

    /// Renames the current Space and advances its synchronized identity revision.
    pub fn rename(&self, spaceName: String) -> Result<CoreSpace, String> {
        validateSpaceName(&spaceName)?;
        let localSpace = self.initialize()?;
        let nextRevision = localSpace
            .spaceRevision
            .checked_add(1)
            .ok_or_else(|| "Device space revision overflow".to_string())?;
        self.writeSpaceProjection(
            localSpace.spaceId,
            spaceName,
            nextRevision,
            localSpace.members.into_iter().collect(),
        )
    }

    /// Leaves the current device space and creates a new single-device space.
    pub fn leave(&self) -> Result<CoreSpace, String> {
        let localSpace = self.initialize()?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let localDeviceProfile = self
            .deviceProfiles()?
            .get(&identity.nodeId)
            .cloned()
            .ok_or_else(|| "Current device profile is not initialized".to_string())?;
        let now = currentTimeMillis();
        let nextRevision = nextSpaceRevision(localSpace.spaceRevision, localSpace.spaceRevision)?;
        self.writeMemberRecord(&CoreSpaceMemberRecord {
            spaceId: newSpaceId(),
            spaceName: localDeviceProfile.displayName,
            spaceRevision: nextRevision,
            nodeId: identity.nodeId,
            joinedAt: now,
            updatedAt: now,
        })?;
        self.space()
    }

    /// Removes selected members by publishing a new local Space projection.
    pub fn removeMembers(&self, removedNodeIds: BTreeSet<String>) -> Result<CoreSpace, String> {
        let localSpace = self.initialize()?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        if removedNodeIds.contains(&identity.nodeId) {
            return Err("The current device cannot be removed from its own Space".to_string());
        }
        let members = localSpace
            .members
            .into_iter()
            .filter(|nodeId| !removedNodeIds.contains(nodeId))
            .collect::<BTreeSet<_>>();
        if members.is_empty() {
            return Err("A device Space must retain the current device".to_string());
        }
        let nextRevision = nextSpaceRevision(localSpace.spaceRevision, localSpace.spaceRevision)?;
        self.writeSpaceProjection(newSpaceId(), localSpace.spaceName, nextRevision, members)
    }

    /// Returns whether the supplied CoreNode is a member of the current Space.
    pub fn contains(&self, nodeId: String) -> Result<bool, String> {
        validateNodeId(&nodeId)?;
        Ok(self.space()?.members.iter().any(|member| member == &nodeId))
    }

    /// Publishes the current device presentation as synchronized device-space metadata.
    #[allow(non_snake_case)]
    pub fn writeLocalDeviceProfile(
        &self,
        displayName: String,
        platform: String,
        model: String,
        coreVersion: String,
    ) -> Result<CoreSpaceDeviceProfile, String> {
        validateDeviceProfileField("display name", &displayName)?;
        validateDeviceProfileField("platform", &platform)?;
        validateDeviceProfileField("model", &model)?;
        validateDeviceProfileField("core version", &coreVersion)?;
        let initialSpace = self.initializeNamed(displayName.clone())?;
        if initialSpace.spaceRevision == 1
            && initialSpace.members.len() == 1
            && initialSpace.spaceName == defaultSpaceName()
            && initialSpace.spaceName != displayName
        {
            self.rename(displayName.clone())?;
        }
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let profiles = self.deviceProfiles()?;
        let profile = match profiles.get(&identity.nodeId) {
            Some(profile)
                if profile.displayName == displayName
                    && profile.platform == platform
                    && profile.model == model
                    && profile.coreVersion.as_deref() == Some(coreVersion.as_str()) =>
            {
                return Ok(profile.clone());
            }
            Some(profile) => CoreSpaceDeviceProfile {
                nodeId: identity.nodeId,
                displayName,
                userName: profile.userName.clone(),
                platform,
                model,
                coreVersion: Some(coreVersion),
                updatedAt: currentTimeMillis(),
            },
            None => CoreSpaceDeviceProfile {
                nodeId: identity.nodeId,
                displayName,
                userName: String::new(),
                platform,
                model,
                coreVersion: Some(coreVersion),
                updatedAt: currentTimeMillis(),
            },
        };
        self.writeDeviceProfile(&profile)?;
        Ok(profile)
    }

    /// Publishes the configured user name owned by the current device identity.
    #[allow(non_snake_case)]
    pub fn writeLocalDeviceUserName(
        &self,
        userName: String,
    ) -> Result<CoreSpaceDeviceProfile, String> {
        validateDeviceUserName(&userName)?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let mut profile = self
            .deviceProfiles()?
            .remove(&identity.nodeId)
            .ok_or_else(|| "Current device profile is not initialized".to_string())?;
        if profile.userName == userName {
            return Ok(profile);
        }
        profile.userName = userName;
        profile.updatedAt = currentTimeMillis();
        self.writeDeviceProfile(&profile)?;
        Ok(profile)
    }

    /// Reads every synchronized device presentation visible in the current device space.
    #[allow(non_snake_case)]
    pub fn deviceProfiles(&self) -> Result<BTreeMap<String, CoreSpaceDeviceProfile>, String> {
        let entries = self
            .storage
            .list(RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH)
            .map_err(|error| error.to_string())?;
        let mut profiles = BTreeMap::new();
        for entry in entries {
            if entry.isDirectory {
                continue;
            }
            let preferences =
                PreferencesDataStore::newWithStorage(self.storage.clone(), entry.path.clone())
                    .data()
                    .map_err(|error| error.to_string())?;
            let encoded = preferences
                .get(&stringPreferencesKey(CORE_SPACE_RECORD_KEY))
                .ok_or_else(|| format!("Device space profile record is empty: {}", entry.path))?;
            let profile: CoreSpaceDeviceProfile =
                serde_json::from_str(encoded).map_err(|error| error.to_string())?;
            validateDeviceProfile(&profile)?;
            profiles.insert(profile.nodeId.clone(), profile);
        }
        Ok(profiles)
    }

    /// Reads the synchronized device presentations for every current Space member.
    #[allow(non_snake_case)]
    pub fn deviceProfilesForCurrentSpace(&self) -> Result<Vec<CoreSpaceDeviceProfile>, String> {
        let space = self.space()?;
        let profiles = self.deviceProfiles()?;
        let mut spaceProfiles = Vec::new();
        for deviceId in space.members {
            let profile = profiles.get(&deviceId).ok_or_else(|| {
                format!("Device profile is missing in the current device space: {deviceId}")
            })?;
            spaceProfiles.push(profile.clone());
        }
        Ok(spaceProfiles)
    }

    /// Imports synchronized device presentations carried by the Space control protocol.
    #[allow(non_snake_case)]
    pub fn importDeviceProfiles(
        &self,
        profiles: Vec<CoreSpaceDeviceProfile>,
    ) -> Result<(), String> {
        for profile in profiles {
            self.writeDeviceProfile(&profile)?;
        }
        Ok(())
    }

    /// Publishes the current device availability as synchronized device-space metadata.
    #[allow(non_snake_case)]
    pub fn writeLocalDevicePresence(
        &self,
        active: bool,
        baseUrl: String,
        tokenHash: String,
        version: String,
    ) -> Result<CoreSpaceDevicePresence, String> {
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let presence = CoreSpaceDevicePresence {
            nodeId: identity.nodeId,
            active,
            baseUrl,
            tokenHash,
            version,
            updatedAt: currentTimeMillis(),
        };
        self.writeDevicePresence(&presence)?;
        Ok(presence)
    }

    /// Imports one observed device availability announcement into synchronized metadata.
    #[allow(non_snake_case)]
    pub fn writeObservedDevicePresence(
        &self,
        presence: CoreSpaceDevicePresence,
    ) -> Result<(), String> {
        self.writeDevicePresence(&presence)
    }

    /// Reads every synchronized device availability announcement visible to this CoreNode.
    #[allow(non_snake_case)]
    pub fn devicePresences(&self) -> Result<BTreeMap<String, CoreSpaceDevicePresence>, String> {
        let entries = self
            .storage
            .list(RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH)
            .map_err(|error| error.to_string())?;
        let mut presences = BTreeMap::new();
        for entry in entries {
            if entry.isDirectory {
                continue;
            }
            let preferences =
                PreferencesDataStore::newWithStorage(self.storage.clone(), entry.path.clone())
                    .data()
                    .map_err(|error| error.to_string())?;
            let encoded = preferences
                .get(&stringPreferencesKey(CORE_SPACE_RECORD_KEY))
                .ok_or_else(|| format!("Device space presence record is empty: {}", entry.path))?;
            let presence: CoreSpaceDevicePresence =
                serde_json::from_str(encoded).map_err(|error| error.to_string())?;
            validateDevicePresence(&presence)?;
            presences.insert(presence.nodeId.clone(), presence);
        }
        Ok(presences)
    }

    /// Replaces the local CoreNode topology announcement from the active Peer Link registry.
    #[allow(non_snake_case)]
    pub fn setDirectPeers(&self, peerNodeIds: Vec<String>) -> Result<(), String> {
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let peers = peerNodeIds.into_iter().collect::<BTreeSet<_>>();
        for peerNodeId in &peers {
            validateNodeId(peerNodeId)?;
            if peerNodeId == &identity.nodeId {
                return Err("A device cannot register itself as a direct peer".to_string());
            }
        }
        self.writeTopologyRecord(&CoreSpaceTopologyRecord {
            nodeId: identity.nodeId,
            peers: peers.iter().cloned().collect(),
            updatedAt: currentTimeMillis(),
        })
    }

    /// Returns every directed direct-device connection inside the current device space.
    #[allow(non_snake_case)]
    pub fn deviceConnections(&self) -> Result<Vec<CoreSpaceDeviceConnection>, String> {
        let members = self.space()?.members.into_iter().collect::<BTreeSet<_>>();
        let mut connections = BTreeSet::new();
        for record in self.topologyRecords()?.into_values() {
            if !members.contains(&record.nodeId) {
                continue;
            }
            for peerDeviceId in record.peers {
                if !members.contains(&peerDeviceId) {
                    continue;
                }
                connections.insert(CoreSpaceDeviceConnection {
                    firstDeviceId: record.nodeId.clone(),
                    secondDeviceId: peerDeviceId,
                });
            }
        }
        Ok(connections.into_iter().collect())
    }

    /// Resolves the direct peer that begins the shortest route to one Space member.
    #[allow(non_snake_case)]
    pub fn nextHop(&self, targetNodeId: String) -> Result<String, String> {
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let topology = self.topologyRecords()?;
        let directPeerNodeIds = topology
            .get(&identity.nodeId)
            .ok_or_else(|| {
                format!(
                    "Device space connections are missing an announcement for {}",
                    identity.nodeId
                )
            })?
            .peers
            .iter()
            .cloned()
            .collect();
        self.nextHopThroughPeers(targetNodeId, directPeerNodeIds)
    }

    /// Resolves the shortest route whose first hop belongs to the supplied active peer set.
    #[allow(non_snake_case)]
    pub fn nextHopThroughPeers(
        &self,
        targetNodeId: String,
        directPeerNodeIds: BTreeSet<String>,
    ) -> Result<String, String> {
        self.reachableNextHopThroughPeers(targetNodeId.clone(), directPeerNodeIds)?
            .ok_or_else(|| {
                format!("Device is not reachable in the current device space: {targetNodeId}")
            })
    }

    /// Resolves an active first hop when the supplied peer set proves the target reachable.
    #[allow(non_snake_case)]
    pub fn reachableNextHopThroughPeers(
        &self,
        targetNodeId: String,
        directPeerNodeIds: BTreeSet<String>,
    ) -> Result<Option<String>, String> {
        validateNodeId(&targetNodeId)?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        if identity.nodeId == targetNodeId {
            return Err("The route target is the current device".to_string());
        }
        if !self.contains(targetNodeId.clone())? {
            return Err(format!(
                "Device is not a member of the current device space: {targetNodeId}"
            ));
        }
        let topology = self.topologyRecords()?;
        let mut queue = std::collections::VecDeque::new();
        let mut visited = BTreeSet::new();
        visited.insert(identity.nodeId.clone());
        let Some(localTopology) = topology.get(&identity.nodeId) else {
            return Ok(None);
        };
        for peerNodeId in localTopology
            .peers
            .iter()
            .filter(|peerNodeId| directPeerNodeIds.contains(*peerNodeId))
        {
            visited.insert(peerNodeId.clone());
            if peerNodeId == &targetNodeId {
                return Ok(Some(peerNodeId.clone()));
            }
            queue.push_back((peerNodeId.clone(), peerNodeId.clone()));
        }
        while let Some((nodeId, firstHop)) = queue.pop_front() {
            let Some(record) = topology.get(&nodeId) else {
                continue;
            };
            for peerNodeId in &record.peers {
                if !visited.insert(peerNodeId.clone()) {
                    continue;
                }
                if peerNodeId == &targetNodeId {
                    return Ok(Some(firstHop));
                }
                queue.push_back((peerNodeId.clone(), firstHop.clone()));
            }
        }
        Ok(None)
    }

    /// Reads every synchronized member record stored by this CoreNode.
    fn memberRecords(&self) -> Result<BTreeMap<String, CoreSpaceMemberRecord>, String> {
        let entries = self
            .storage
            .list(RUNTIME_SPACE_MEMBERS_DIR_PATH)
            .map_err(|error| error.to_string())?;
        let mut records = BTreeMap::new();
        for entry in entries {
            if entry.isDirectory {
                continue;
            }
            let preferences =
                PreferencesDataStore::newWithStorage(self.storage.clone(), entry.path.clone())
                    .data()
                    .map_err(|error| error.to_string())?;
            let encoded = preferences
                .get(&stringPreferencesKey(CORE_SPACE_RECORD_KEY))
                .ok_or_else(|| format!("Device space member record is empty: {}", entry.path))?;
            let record: CoreSpaceMemberRecord =
                serde_json::from_str(encoded).map_err(|error| error.to_string())?;
            validateMemberRecord(&record)?;
            records.insert(record.nodeId.clone(), record);
        }
        Ok(records)
    }

    /// Writes one member as an independently synchronized preferences entity.
    fn writeMemberRecord(&self, record: &CoreSpaceMemberRecord) -> Result<(), String> {
        validateMemberRecord(record)?;
        let mut preferences = emptyPreferences();
        preferences.set(
            &stringPreferencesKey(CORE_SPACE_RECORD_KEY),
            serde_json::to_string(record).map_err(|error| error.to_string())?,
        );
        self.syncedMemberStore(&record.nodeId)
            .replace(preferences)
            .map_err(|error| error.to_string())
    }

    /// Writes one complete membership projection under a shared Space identity revision.
    #[allow(non_snake_case)]
    fn writeSpaceProjection(
        &self,
        spaceId: String,
        spaceName: String,
        spaceRevision: i64,
        members: BTreeSet<String>,
    ) -> Result<CoreSpace, String> {
        validateSpaceName(&spaceName)?;
        if spaceRevision <= 0 {
            return Err("Device space revision must be greater than zero".to_string());
        }
        let existingRecords = self.memberRecords()?;
        let now = currentTimeMillis();
        for nodeId in members {
            validateNodeId(&nodeId)?;
            let joinedAt = existingRecords
                .get(&nodeId)
                .map(|record| record.joinedAt)
                .unwrap_or(now);
            let record = CoreSpaceMemberRecord {
                spaceId: spaceId.clone(),
                spaceName: spaceName.clone(),
                spaceRevision,
                nodeId,
                joinedAt,
                updatedAt: now,
            };
            if existingRecords
                .get(&record.nodeId)
                .map(|existing| existing.hasSameMembershipAs(&record))
                .unwrap_or(false)
            {
                continue;
            }
            self.writeMemberRecord(&record)?;
        }
        self.space()
    }

    /// Reads every synchronized topology announcement visible to this CoreNode.
    fn topologyRecords(&self) -> Result<BTreeMap<String, CoreSpaceTopologyRecord>, String> {
        let entries = self
            .storage
            .list(RUNTIME_SPACE_TOPOLOGY_DIR_PATH)
            .map_err(|error| error.to_string())?;
        let mut records = BTreeMap::new();
        for entry in entries {
            if entry.isDirectory {
                continue;
            }
            let preferences =
                PreferencesDataStore::newWithStorage(self.storage.clone(), entry.path.clone())
                    .data()
                    .map_err(|error| error.to_string())?;
            let encoded = preferences
                .get(&stringPreferencesKey(CORE_SPACE_RECORD_KEY))
                .ok_or_else(|| {
                    format!("Device space connection record is empty: {}", entry.path)
                })?;
            let record: CoreSpaceTopologyRecord =
                serde_json::from_str(encoded).map_err(|error| error.to_string())?;
            validateTopologyRecord(&record)?;
            records.insert(record.nodeId.clone(), record);
        }
        Ok(records)
    }

    /// Writes the local CoreNode topology announcement as a synchronized preferences entity.
    fn writeTopologyRecord(&self, record: &CoreSpaceTopologyRecord) -> Result<(), String> {
        validateTopologyRecord(record)?;
        let mut preferences = emptyPreferences();
        preferences.set(
            &stringPreferencesKey(CORE_SPACE_RECORD_KEY),
            serde_json::to_string(record).map_err(|error| error.to_string())?,
        );
        PreferencesDataStore::newWithStorage(
            self.storage.clone(),
            format!(
                "{RUNTIME_SPACE_TOPOLOGY_DIR_PATH}/{}.preferences.json",
                record.nodeId
            ),
        )
        .replace(preferences)
        .map_err(|error| error.to_string())
    }

    /// Writes one synchronized device availability announcement owned by its device identity.
    #[allow(non_snake_case)]
    fn writeDevicePresence(&self, presence: &CoreSpaceDevicePresence) -> Result<(), String> {
        validateDevicePresence(presence)?;
        let mut preferences = emptyPreferences();
        preferences.set(
            &stringPreferencesKey(CORE_SPACE_RECORD_KEY),
            serde_json::to_string(presence).map_err(|error| error.to_string())?,
        );
        PreferencesDataStore::newWithStorage(
            self.storage.clone(),
            format!(
                "{RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH}/{}.preferences.json",
                presence.nodeId
            ),
        )
        .replace(preferences)
        .map_err(|error| error.to_string())
    }

    /// Writes one synchronized device presentation owned by its device identity.
    #[allow(non_snake_case)]
    fn writeDeviceProfile(&self, profile: &CoreSpaceDeviceProfile) -> Result<(), String> {
        validateDeviceProfile(profile)?;
        let mut preferences = emptyPreferences();
        preferences.set(
            &stringPreferencesKey(CORE_SPACE_RECORD_KEY),
            serde_json::to_string(profile).map_err(|error| error.to_string())?,
        );
        PreferencesDataStore::newWithStorage(
            self.storage.clone(),
            format!(
                "{RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH}/{}.preferences.json",
                profile.nodeId
            ),
        )
        .replace(preferences)
        .map_err(|error| error.to_string())
    }

    /// Creates the synchronized preferences store for one Space member.
    fn syncedMemberStore(&self, nodeId: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(
            self.storage.clone(),
            format!("{RUNTIME_SPACE_MEMBERS_DIR_PATH}/{nodeId}.preferences.json"),
        )
    }
}

impl CoreSpaceMemberRecord {
    /// Returns whether two records encode the same durable membership fact.
    #[allow(non_snake_case)]
    fn hasSameMembershipAs(&self, other: &Self) -> bool {
        self.spaceId == other.spaceId
            && self.spaceName == other.spaceName
            && self.spaceRevision == other.spaceRevision
            && self.nodeId == other.nodeId
            && self.joinedAt == other.joinedAt
    }
}

/// Builds one canonical Space projection from synchronized member records.
fn coreSpaceFromRecords(
    records: BTreeMap<String, CoreSpaceMemberRecord>,
    localNodeId: &str,
) -> Result<CoreSpace, String> {
    if records.is_empty() {
        return Err("Device space is not initialized".to_string());
    }
    let identity = records
        .get(localNodeId)
        .ok_or_else(|| format!("Device space has no membership record for {localNodeId}"))?;
    let members = records
        .values()
        .filter(|record| record.spaceId == identity.spaceId)
        .map(|record| record.nodeId.clone())
        .collect();
    Ok(CoreSpace {
        spaceId: identity.spaceId.clone(),
        spaceName: identity.spaceName.clone(),
        spaceRevision: identity.spaceRevision,
        members,
    })
}

/// Validates one complete Space projection received from a paired peer.
fn validateCoreSpace(space: &CoreSpace) -> Result<(), String> {
    if space.spaceId.trim().is_empty() {
        return Err("Device space id must not be empty".to_string());
    }
    validateSpaceName(&space.spaceName)?;
    if space.spaceRevision <= 0 {
        return Err("Device space revision must be greater than zero".to_string());
    }
    if space.members.is_empty() {
        return Err("Device space must include at least one device".to_string());
    }
    for nodeId in &space.members {
        validateNodeId(nodeId)?;
    }
    Ok(())
}

/// Validates one persisted Space membership entity.
fn validateMemberRecord(record: &CoreSpaceMemberRecord) -> Result<(), String> {
    if record.spaceId.trim().is_empty() {
        return Err("Device space member record is missing its id".to_string());
    }
    validateSpaceName(&record.spaceName)?;
    if record.spaceRevision <= 0 {
        return Err("Device space member record has an invalid revision".to_string());
    }
    validateNodeId(&record.nodeId)
}

/// Validates one synchronized device presentation without deriving values from its text.
#[allow(non_snake_case)]
fn validateDeviceProfile(profile: &CoreSpaceDeviceProfile) -> Result<(), String> {
    validateNodeId(&profile.nodeId)?;
    validateDeviceProfileField("display name", &profile.displayName)?;
    validateDeviceUserName(&profile.userName)?;
    validateDeviceProfileField("platform", &profile.platform)?;
    validateDeviceProfileField("model", &profile.model)?;
    if let Some(coreVersion) = &profile.coreVersion {
        validateDeviceProfileField("core version", coreVersion)?;
    }
    if profile.updatedAt <= 0 {
        return Err("Device space profile has an invalid update timestamp".to_string());
    }
    Ok(())
}

/// Validates one synchronized device availability announcement.
#[allow(non_snake_case)]
fn validateDevicePresence(presence: &CoreSpaceDevicePresence) -> Result<(), String> {
    validateNodeId(&presence.nodeId)?;
    if presence.active {
        validateDevicePresenceField("base URL", &presence.baseUrl)?;
        validateDevicePresenceField("token hash", &presence.tokenHash)?;
        validateDevicePresenceField("version", &presence.version)?;
    }
    if presence.updatedAt <= 0 {
        return Err("Device space presence has an invalid update timestamp".to_string());
    }
    Ok(())
}

/// Validates one active device availability field.
#[allow(non_snake_case)]
fn validateDevicePresenceField(fieldName: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "Device space presence {fieldName} must not be empty"
        ));
    }
    if trimmed.chars().count() > 512 {
        return Err(format!(
            "Device space presence {fieldName} must not exceed 512 characters"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!(
            "Device space presence {fieldName} must not contain control characters"
        ));
    }
    Ok(())
}

/// Validates one optional user-configured name published by a device.
#[allow(non_snake_case)]
fn validateDeviceUserName(userName: &str) -> Result<(), String> {
    let trimmed = userName.trim();
    if trimmed.chars().count() > 80 {
        return Err("Device user name must not exceed 80 characters".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("Device user name must not contain control characters".to_string());
    }
    Ok(())
}

/// Validates one required device presentation field.
#[allow(non_snake_case)]
fn validateDeviceProfileField(fieldName: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "Device space profile {fieldName} must not be empty"
        ));
    }
    if trimmed.chars().count() > 160 {
        return Err(format!(
            "Device space profile {fieldName} must not exceed 160 characters"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!(
            "Device space profile {fieldName} must not contain control characters"
        ));
    }
    Ok(())
}

/// Returns the initial display name assigned to a newly created Space.
#[allow(non_snake_case)]
fn defaultSpaceName() -> String {
    "Operit".to_string()
}

/// Creates a new identity for a device space created after leaving another space.
#[allow(non_snake_case)]
fn newSpaceId() -> String {
    format!("space-{}", Uuid::new_v4().simple())
}

/// Calculates the next identity revision for a directional Space join.
#[allow(non_snake_case)]
fn nextSpaceRevision(localRevision: i64, peerRevision: i64) -> Result<i64, String> {
    localRevision
        .max(peerRevision)
        .checked_add(1)
        .ok_or_else(|| "Device space revision overflow".to_string())
}

/// Validates one user-visible Space name.
#[allow(non_snake_case)]
fn validateSpaceName(spaceName: &str) -> Result<(), String> {
    let trimmed = spaceName.trim();
    if trimmed.is_empty() {
        return Err("Device space name must not be empty".to_string());
    }
    if trimmed.chars().count() > 80 {
        return Err("Device space name must not exceed 80 characters".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("Device space name must not contain control characters".to_string());
    }
    Ok(())
}

/// Validates one synchronized CoreNode topology announcement.
fn validateTopologyRecord(record: &CoreSpaceTopologyRecord) -> Result<(), String> {
    validateNodeId(&record.nodeId)?;
    for peerNodeId in &record.peers {
        validateNodeId(peerNodeId)?;
        if peerNodeId == &record.nodeId {
            return Err("Device space connections cannot contain a self edge".to_string());
        }
    }
    Ok(())
}

/// Validates one CoreNode identifier used by Space membership.
fn validateNodeId(nodeId: &str) -> Result<(), String> {
    if nodeId.trim().is_empty() {
        return Err("Device id must not be empty".to_string());
    }
    if nodeId
        .chars()
        .any(|character| character == '/' || character == '\\' || character == '\0')
    {
        return Err("Device id contains an invalid path character".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use operit_host_api::{HostError, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

    use crate::SyncOperationStore::SyncOperationStore;

    #[derive(Clone, Default)]
    struct MemoryStorageHost {
        files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        /// Returns no physical runtime root for the in-memory test host.
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }

        /// Returns no physical workspace root for the in-memory test host.
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }

        /// Reads one exact virtual file from the in-memory test host.
        fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new(format!("missing runtime storage file: {path}")))
        }

        /// Reads one byte range from the in-memory test host.
        fn readBytesRange(
            &self,
            path: &str,
            offset: u64,
            length: usize,
        ) -> operit_host_api::HostResult<Vec<u8>> {
            let content = self.readBytes(path)?;
            let start = usize::try_from(offset)
                .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
            if start >= content.len() {
                return Ok(Vec::new());
            }
            let end = start
                .checked_add(length)
                .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
                .min(content.len());
            Ok(content[start..end].to_vec())
        }

        /// Writes one complete virtual file to the in-memory test host.
        fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .insert(path.to_string(), content.to_vec());
            Ok(())
        }

        /// Appends bytes to one virtual file in the in-memory test host.
        fn appendBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .entry(path.to_string())
                .or_default()
                .extend_from_slice(content);
            Ok(())
        }

        /// Deletes one virtual file from the in-memory test host.
        fn delete(&self, path: &str, _recursive: bool) -> operit_host_api::HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(path);
            Ok(())
        }

        /// Reports whether one virtual file exists in the in-memory test host.
        fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .contains_key(path))
        }

        /// Lists virtual files stored under one prefix in the in-memory test host.
        fn list(&self, prefix: &str) -> operit_host_api::HostResult<Vec<RuntimeStorageEntry>> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .iter()
                .filter(|(path, _)| path.starts_with(prefix))
                .map(|(path, content)| RuntimeStorageEntry {
                    path: path.clone(),
                    isDirectory: false,
                    size: content.len() as i64,
                })
                .collect())
        }
    }

    /// Reads the highest local sync sequence stored by one memory host.
    fn localSyncSequence(host: Arc<dyn RuntimeStorageHost>) -> i64 {
        SyncOperationStore::new(host, RUNTIME_SYNC_DIR_PATH.to_string())
            .localClock()
            .expect("test sync clock must be readable")
            .sequences
            .values()
            .copied()
            .max()
            .unwrap_or(0)
    }

    /// Builds one deterministic peer Space projection for idempotence tests.
    fn peerSpace(peerNodeId: &str) -> CoreSpace {
        CoreSpace {
            spaceId: "space-peer".to_string(),
            spaceName: "peer-space".to_string(),
            spaceRevision: 7,
            members: vec![peerNodeId.to_string()],
        }
    }

    /// Verifies that repeating the same paired Space observation records no new transaction.
    #[test]
    fn observe_paired_space_is_idempotent_for_identical_membership() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = CoreSpaceStore::new(host.clone());
        store
            .initializeNamed("local-space".to_string())
            .expect("test space must initialize");
        let peerNodeId = "core-peer-observe";
        store
            .observePairedDeviceSpace(peerNodeId.to_string(), peerSpace(peerNodeId))
            .expect("first peer space observation must succeed");
        let sequenceAfterFirstObserve = localSyncSequence(host.clone());

        store
            .observePairedDeviceSpace(peerNodeId.to_string(), peerSpace(peerNodeId))
            .expect("second peer space observation must succeed");

        assert_eq!(localSyncSequence(host), sequenceAfterFirstObserve);
    }

    /// Verifies that adopting the same joined Space projection records no new transaction.
    #[test]
    fn adopt_joined_space_is_idempotent_for_identical_membership() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = CoreSpaceStore::new(host.clone());
        let localSpace = store
            .initializeNamed("local-space".to_string())
            .expect("test space must initialize");
        let joinedSpace = CoreSpace {
            spaceId: localSpace.spaceId,
            spaceName: localSpace.spaceName,
            spaceRevision: localSpace.spaceRevision,
            members: localSpace.members,
        };
        store
            .adopt(joinedSpace.clone())
            .expect("first joined space adoption must succeed");
        let sequenceAfterFirstAdopt = localSyncSequence(host.clone());

        store
            .adopt(joinedSpace)
            .expect("second joined space adoption must succeed");

        assert_eq!(localSyncSequence(host), sequenceAfterFirstAdopt);
    }
}
