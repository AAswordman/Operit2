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
const UNMEASURED_DIRECT_PEER_COST: u64 = 1_000_000_000;

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

/// Publishes the measured quality of one directed active Peer Link.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSpaceLinkAdvertisement {
    pub targetNodeId: String,
    pub channelEpoch: String,
    pub sequence: u64,
    pub measuredAt: i64,
    pub expiresAt: i64,
    pub smoothedRttMs: u64,
    pub lossPermille: u16,
    pub congestionPermille: u16,
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
    links: Vec<CoreSpaceLinkAdvertisement>,
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
        let merged = self.mergedProjection(peerSpace)?;
        self.writeSpaceProjection(
            merged.spaceId,
            merged.spaceName,
            merged.spaceRevision,
            merged.members.into_iter().collect(),
        )
    }

    /// Builds the exact membership projection proposed by joining the current Space to one peer.
    #[allow(non_snake_case)]
    pub fn mergedProjection(&self, peerSpace: CoreSpace) -> Result<CoreSpace, String> {
        validateCoreSpace(&peerSpace)?;
        let localSpace = self.initialize()?;
        Ok(CoreSpace {
            spaceId: peerSpace.spaceId,
            spaceName: peerSpace.spaceName,
            spaceRevision: nextSpaceRevision(localSpace.spaceRevision, peerSpace.spaceRevision)?,
            members: localSpace
                .members
                .into_iter()
                .chain(peerSpace.members)
                .collect(),
        })
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
        let peerSet = peerNodeIds.into_iter().collect::<BTreeSet<_>>();
        for peerNodeId in &peerSet {
            validateNodeId(peerNodeId)?;
            if peerNodeId == &identity.nodeId {
                return Err("A device cannot register itself as a direct peer".to_string());
            }
        }
        let peers = peerSet.into_iter().collect::<Vec<_>>();
        let topology = self.topologyRecords()?;
        let links = topology
            .get(&identity.nodeId)
            .map(|record| {
                record
                    .links
                    .iter()
                    .filter(|link| peers.contains(&link.targetNodeId))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        self.writeTopologyRecord(&CoreSpaceTopologyRecord {
            nodeId: identity.nodeId.clone(),
            peers: peers.clone(),
            links,
            updatedAt: currentTimeMillis(),
        })
    }

    /// Publishes the newest locally measured quality for one active direct Peer Link.
    #[allow(non_snake_case)]
    pub fn publishLocalLinkAdvertisement(
        &self,
        advertisement: CoreSpaceLinkAdvertisement,
    ) -> Result<(), String> {
        validateLinkAdvertisement(&advertisement)?;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        if advertisement.targetNodeId == identity.nodeId {
            return Err("A device cannot publish a link to itself".to_string());
        }
        let topology = self.topologyRecords()?;
        let Some(record) = topology.get(&identity.nodeId) else {
            return Err("Local device topology is not initialized".to_string());
        };
        let peers = record.peers.clone();
        if !peers.iter().any(|peer| peer == &advertisement.targetNodeId) {
            return Err("Link advertisement target is not an active direct peer".to_string());
        }
        let mut links = record.links.clone();
        links.retain(|link| link.targetNodeId != advertisement.targetNodeId);
        links.push(advertisement);
        links.sort_by(|left, right| left.targetNodeId.cmp(&right.targetNodeId));
        self.writeTopologyRecord(&CoreSpaceTopologyRecord {
            nodeId: identity.nodeId,
            peers,
            links,
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

    /// Resolves an active first hop with Dijkstra over non-expired directed link measurements.
    #[allow(non_snake_case)]
    pub fn reachableNextHopThroughPeers(
        &self,
        targetNodeId: String,
        directPeerNodeIds: BTreeSet<String>,
    ) -> Result<Option<String>, String> {
        let transitNodeIds = self.space()?.members.into_iter().collect();
        self.reachableNextHopThroughPeersWithTransitNodes(
            targetNodeId,
            directPeerNodeIds,
            transitNodeIds,
        )
    }

    /// Resolves a weighted first hop while allowing only selected members to forward a later hop.
    #[allow(non_snake_case)]
    pub fn reachableNextHopThroughPeersWithTransitNodes(
        &self,
        targetNodeId: String,
        directPeerNodeIds: BTreeSet<String>,
        transitNodeIds: BTreeSet<String>,
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
        let members = self.space()?.members.into_iter().collect::<BTreeSet<_>>();
        let topology = self.topologyRecords()?;
        let now = currentTimeMillis();
        let mut distances = BTreeMap::<String, u64>::new();
        let mut firstHops = BTreeMap::<String, String>::new();
        let mut settled = BTreeSet::<String>::new();
        distances.insert(identity.nodeId.clone(), 0);
        let localTopology = topology.get(&identity.nodeId);
        for peerNodeId in &directPeerNodeIds {
            if peerNodeId == &identity.nodeId || !members.contains(peerNodeId) {
                continue;
            }
            let measuredCost = localTopology
                .and_then(|record| {
                    record
                        .links
                        .iter()
                        .find(|link| link.targetNodeId == *peerNodeId && link.expiresAt > now)
                })
                .map(linkCost);
            distances.insert(
                peerNodeId.clone(),
                measuredCost.unwrap_or(UNMEASURED_DIRECT_PEER_COST),
            );
            firstHops.insert(peerNodeId.clone(), peerNodeId.clone());
        }

        loop {
            let next = distances
                .iter()
                .filter(|(nodeId, _)| !settled.contains(*nodeId))
                .filter_map(|(nodeId, cost)| {
                    let firstHop = match firstHops.get(nodeId) {
                        Some(firstHop) => firstHop.clone(),
                        None if nodeId == &identity.nodeId => String::new(),
                        None => return None,
                    };
                    Some((nodeId.clone(), *cost, firstHop))
                })
                .min_by(|left, right| {
                    left.1
                        .cmp(&right.1)
                        .then(left.2.cmp(&right.2))
                        .then(left.0.cmp(&right.0))
                });
            let Some((nodeId, cost, firstHop)) = next else {
                return Ok(None);
            };
            if nodeId == targetNodeId {
                return Ok(firstHops.get(&nodeId).cloned());
            }
            settled.insert(nodeId.clone());
            if nodeId != identity.nodeId && !transitNodeIds.contains(&nodeId) {
                continue;
            }
            let Some(topologyRecord) = topology.get(&nodeId) else {
                continue;
            };
            for link in &topologyRecord.links {
                if link.expiresAt <= now
                    || !members.contains(&link.targetNodeId)
                    || !topologyRecord.peers.contains(&link.targetNodeId)
                {
                    continue;
                }
                if nodeId == identity.nodeId && !directPeerNodeIds.contains(&link.targetNodeId) {
                    continue;
                }
                if settled.contains(&link.targetNodeId) {
                    continue;
                }
                let candidateFirstHop = if nodeId == identity.nodeId {
                    link.targetNodeId.clone()
                } else {
                    firstHop.clone()
                };
                let candidateCost = cost.saturating_add(linkCost(link));
                let replace = match distances.get(&link.targetNodeId) {
                    Some(existingCost) if *existingCost < candidateCost => false,
                    Some(existingCost) if *existingCost == candidateCost => firstHops
                        .get(&link.targetNodeId)
                        .map(|existingFirstHop| candidateFirstHop < *existingFirstHop)
                        .unwrap_or(true),
                    _ => true,
                };
                if replace {
                    distances.insert(link.targetNodeId.clone(), candidateCost);
                    firstHops.insert(link.targetNodeId.clone(), candidateFirstHop);
                }
            }
        }
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
    let peers = record.peers.iter().cloned().collect::<BTreeSet<_>>();
    if peers.len() != record.peers.len() {
        return Err("Device space topology has duplicate direct peers".to_string());
    }
    for peerNodeId in &record.peers {
        validateNodeId(peerNodeId)?;
        if peerNodeId == &record.nodeId {
            return Err("Device space connections cannot contain a self edge".to_string());
        }
    }
    let mut targets = BTreeSet::new();
    for link in &record.links {
        validateLinkAdvertisement(link)?;
        if link.targetNodeId == record.nodeId {
            return Err("Device space link metrics cannot contain a self edge".to_string());
        }
        if !peers.contains(&link.targetNodeId) {
            return Err("Device space link advertisement has no direct peer edge".to_string());
        }
        if !targets.insert(link.targetNodeId.clone()) {
            return Err("Device space topology has duplicate directed links".to_string());
        }
    }
    Ok(())
}

/// Validates one directed Peer Link quality observation before it enters route selection.
fn validateLinkAdvertisement(advertisement: &CoreSpaceLinkAdvertisement) -> Result<(), String> {
    validateNodeId(&advertisement.targetNodeId)?;
    if advertisement.channelEpoch.trim().is_empty() {
        return Err("Device space link advertisement channel epoch must not be empty".to_string());
    }
    if advertisement.sequence == 0 {
        return Err("Device space link advertisement sequence must be positive".to_string());
    }
    if advertisement.measuredAt <= 0 || advertisement.expiresAt <= advertisement.measuredAt {
        return Err("Device space link advertisement has an invalid lifetime".to_string());
    }
    if advertisement.lossPermille > 1000 || advertisement.congestionPermille > 1000 {
        return Err("Device space link advertisement metric exceeds one permille".to_string());
    }
    Ok(())
}

/// Calculates the integer route cost for one directed Peer Link measurement.
fn linkCost(link: &CoreSpaceLinkAdvertisement) -> u64 {
    1_000_u64
        .saturating_add(link.smoothedRttMs.saturating_mul(10))
        .saturating_add(u64::from(link.lossPermille).saturating_mul(25))
        .saturating_add(u64::from(link.congestionPermille).saturating_mul(10))
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

    use crate::NetworkControlStore::{
        NetworkControlCommand, NetworkControlCommandRecord, NetworkControlIdentityAssignment,
        NetworkControlRole, NetworkControlStore, NETWORK_CONTROL_SYNC_DOMAIN,
    };
    use crate::SyncOperationStore::SyncOperationStore;
    use crate::SyncOperationStore::{NewSyncOperation, SyncOperationSemantics};

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

    /// Builds one non-expired directed link measurement for route selection tests.
    fn testLink(targetNodeId: &str, rttMs: u64) -> CoreSpaceLinkAdvertisement {
        let now = currentTimeMillis();
        CoreSpaceLinkAdvertisement {
            targetNodeId: targetNodeId.to_string(),
            channelEpoch: format!("test-channel-{targetNodeId}"),
            sequence: 1,
            measuredAt: now,
            expiresAt: now + 60_000,
            smoothedRttMs: rttMs,
            lossPermille: 0,
            congestionPermille: 0,
        }
    }

    /// Writes a directed topology record used only by graph algorithm tests.
    fn writeTestTopology(
        store: &CoreSpaceStore,
        nodeId: &str,
        links: Vec<CoreSpaceLinkAdvertisement>,
    ) {
        let peers = links
            .iter()
            .map(|link| link.targetNodeId.clone())
            .collect::<Vec<_>>();
        store
            .writeTopologyRecord(&CoreSpaceTopologyRecord {
                nodeId: nodeId.to_string(),
                peers,
                links,
                updatedAt: currentTimeMillis(),
            })
            .expect("test topology record must write");
    }

    /// Adds one member to the current Space projection for control-policy tests.
    fn addTestSpaceMember(store: &CoreSpaceStore, nodeId: &str) {
        let space = store
            .initialize()
            .expect("test Space must initialize before adding a member");
        let members = space
            .members
            .into_iter()
            .chain(std::iter::once(nodeId.to_string()))
            .collect::<BTreeSet<_>>();
        store
            .writeSpaceProjection(
                space.spaceId,
                space.spaceName,
                space.spaceRevision + 1,
                members,
            )
            .expect("test Space member projection must write");
        store
            .writeDeviceProfile(&CoreSpaceDeviceProfile {
                nodeId: nodeId.to_string(),
                displayName: format!("Device {nodeId}"),
                userName: String::new(),
                platform: "test".to_string(),
                model: "test".to_string(),
                coreVersion: Some("test".to_string()),
                updatedAt: currentTimeMillis(),
            })
            .expect("test member device profile must write");
    }

    /// Initializes the local device presentation required by control audit labels.
    fn initializeTestDeviceProfile(store: &CoreSpaceStore) {
        store
            .writeLocalDeviceProfile(
                "Test device".to_string(),
                "test".to_string(),
                "test".to_string(),
                "test".to_string(),
            )
            .expect("test device profile must initialize");
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

    /// Verifies that the weighted router prefers a lower-cost two-hop path over a slow direct link.
    #[test]
    fn weighted_route_prefers_low_latency_multi_hop_path() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = CoreSpaceStore::new(host.clone());
        let localSpace = store
            .initializeNamed("local-space".to_string())
            .expect("test space must initialize");
        let localNodeId = CoreNodeIdentityStore::new(host)
            .initialize()
            .expect("test node identity must initialize")
            .nodeId;
        let relayNodeId = "node-relay";
        let targetNodeId = "node-target";
        store
            .writeSpaceProjection(
                localSpace.spaceId,
                localSpace.spaceName,
                localSpace.spaceRevision,
                BTreeSet::from([
                    localNodeId.clone(),
                    relayNodeId.to_string(),
                    targetNodeId.to_string(),
                ]),
            )
            .expect("test space projection must include route nodes");
        writeTestTopology(
            &store,
            &localNodeId,
            vec![testLink(targetNodeId, 300), testLink(relayNodeId, 10)],
        );
        writeTestTopology(&store, relayNodeId, vec![testLink(targetNodeId, 10)]);

        let nextHop = store
            .reachableNextHopThroughPeers(
                targetNodeId.to_string(),
                BTreeSet::from([targetNodeId.to_string(), relayNodeId.to_string()]),
            )
            .expect("weighted path must resolve");

        assert_eq!(nextHop.as_deref(), Some(relayNodeId));
    }

    /// Verifies that an active direct peer is reachable before its first measurement arrives.
    #[test]
    fn active_direct_peer_is_reachable_without_measurement() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = CoreSpaceStore::new(host.clone());
        let localSpace = store
            .initializeNamed("local-space".to_string())
            .expect("test space must initialize");
        let localNodeId = CoreNodeIdentityStore::new(host)
            .initialize()
            .expect("test node identity must initialize")
            .nodeId;
        let targetNodeId = "node-unmeasured";
        store
            .writeSpaceProjection(
                localSpace.spaceId,
                localSpace.spaceName,
                localSpace.spaceRevision,
                BTreeSet::from([localNodeId.clone(), targetNodeId.to_string()]),
            )
            .expect("test space projection must include target");
        store
            .setDirectPeers(vec![targetNodeId.to_string()])
            .expect("test direct peer must persist");

        let route = store
            .reachableNextHopThroughPeers(
                targetNodeId.to_string(),
                BTreeSet::from([targetNodeId.to_string()]),
            )
            .expect("unmeasured graph lookup must succeed");

        assert_eq!(route.as_deref(), Some(targetNodeId));
    }

    /// Verifies a non-relay device cannot be selected as a multi-hop forwarding node.
    #[test]
    fn weighted_route_requires_relay_transit_capability() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = CoreSpaceStore::new(host.clone());
        let localSpace = store
            .initializeNamed("relay-space".to_string())
            .expect("test space must initialize");
        let localNodeId = CoreNodeIdentityStore::new(host)
            .initialize()
            .expect("test node identity must initialize")
            .nodeId;
        let ordinaryNodeId = "node-ordinary";
        let relayNodeId = "node-relay";
        let targetNodeId = "node-target";
        store
            .writeSpaceProjection(
                localSpace.spaceId,
                localSpace.spaceName,
                localSpace.spaceRevision,
                BTreeSet::from([
                    localNodeId.clone(),
                    ordinaryNodeId.to_string(),
                    relayNodeId.to_string(),
                    targetNodeId.to_string(),
                ]),
            )
            .expect("test space projection must include route nodes");
        writeTestTopology(
            &store,
            &localNodeId,
            vec![testLink(ordinaryNodeId, 5), testLink(relayNodeId, 20)],
        );
        writeTestTopology(&store, ordinaryNodeId, vec![testLink(targetNodeId, 5)]);
        writeTestTopology(&store, relayNodeId, vec![testLink(targetNodeId, 20)]);

        let nextHop = store
            .reachableNextHopThroughPeersWithTransitNodes(
                targetNodeId.to_string(),
                BTreeSet::from([ordinaryNodeId.to_string(), relayNodeId.to_string()]),
                BTreeSet::from([relayNodeId.to_string()]),
            )
            .expect("constrained weighted route lookup must succeed");

        assert_eq!(nextHop.as_deref(), Some(relayNodeId));
    }

    /// Verifies custom roles, delegated capabilities, command auditing, and member revocation.
    #[test]
    fn network_control_authorizes_custom_roles_and_audits_rejected_commands() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        let space = spaceStore
            .initializeNamed("control-space".to_string())
            .expect("test Space must initialize");
        initializeTestDeviceProfile(&spaceStore);
        let control =
            NetworkControlStore::new(host.clone()).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        addTestSpaceMember(&spaceStore, "peer-archive");
        control
            .admitMember("peer-archive".to_string())
            .expect("administrator must admit the test member");
        control
            .defineRole(NetworkControlRole {
                roleId: "archive_operator".to_string(),
                displayName: "Archive operator".to_string(),
                capabilities: BTreeSet::from(["storage.provide".to_string()]),
            })
            .expect("administrator must define a custom role");
        control
            .setIdentity(NetworkControlIdentityAssignment {
                nodeId: "peer-archive".to_string(),
                roleId: "archive_operator".to_string(),
            })
            .expect("administrator must grant a custom role");
        assert!(control
            .nodeHasCapability("peer-archive", "storage.provide", None)
            .expect("custom capability query must succeed"));

        let rejected = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH)
            .appendLocalOperation(
                "peer-untrusted",
                NewSyncOperation {
                    domain: NETWORK_CONTROL_SYNC_DOMAIN.to_string(),
                    entityType: "command".to_string(),
                    entityId: "untrusted-role".to_string(),
                    operation: "apply".to_string(),
                    semantics: SyncOperationSemantics::Transaction,
                    payload: serde_json::to_value(NetworkControlCommandRecord {
                        commandId: "untrusted-role".to_string(),
                        spaceId: space.spaceId,
                        issuerNodeId: "peer-untrusted".to_string(),
                        command: NetworkControlCommand::DefineRole {
                            role: NetworkControlRole {
                                roleId: "forged".to_string(),
                                displayName: "Forged".to_string(),
                                capabilities: BTreeSet::from(["network.relay".to_string()]),
                            },
                        },
                    })
                    .expect("untrusted command must serialize"),
                },
            )
            .expect("untrusted command must be present for audit replay");
        control
            .applySyncedOperation(&rejected)
            .expect("received control command must append for audit replay");
        assert!(control
            .audit()
            .expect("control audit must materialize")
            .iter()
            .any(|entry| entry.commandId == "untrusted-role" && !entry.accepted));

        control
            .removeMember("peer-archive".to_string())
            .expect("administrator must revoke a member");
        assert!(control
            .nodeIsRemoved("peer-archive")
            .expect("member removal must materialize"));
        assert!(!control
            .nodeHasCapability("peer-archive", "storage.provide", None)
            .expect("revoked capability query must succeed"));
    }

    /// Verifies administrator admission restores a device intentionally disconnected by policy.
    #[test]
    fn network_control_admission_clears_disconnected_device_state() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        initializeTestDeviceProfile(&spaceStore);
        let control =
            NetworkControlStore::new(host).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        addTestSpaceMember(&spaceStore, "peer-rejoin");
        control
            .admitMember("peer-rejoin".to_string())
            .expect("administrator must admit the test member");
        control
            .disconnectNode("peer-rejoin".to_string())
            .expect("administrator must disconnect a device");
        assert!(control
            .nodeIsDisconnected("peer-rejoin")
            .expect("disconnection state must materialize"));

        control
            .admitMember("peer-rejoin".to_string())
            .expect("administrator must readmit a disconnected device");

        assert!(!control
            .nodeIsDisconnected("peer-rejoin")
            .expect("admission must clear disconnection state"));
    }

    /// Verifies a newly admitted member is ordinary by default and cannot execute runtime work.
    #[test]
    fn network_control_admission_assigns_only_the_default_user_role() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        initializeTestDeviceProfile(&spaceStore);
        let control =
            NetworkControlStore::new(host).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        addTestSpaceMember(&spaceStore, "peer-member");
        control
            .admitMember("peer-member".to_string())
            .expect("administrator must admit a member");

        assert!(control
            .nodeHasCapability("peer-member", "network.user", None)
            .expect("default user capability must materialize"));
        assert!(!control
            .nodeHasCapability("peer-member", "runtime.execute", None)
            .expect("ordinary member execution capability must materialize"));
        assert!(!control
            .nodeHasCapability("peer-member", "network.relay", None)
            .expect("ordinary member relay capability must materialize"));
    }

    /// Verifies removal revokes elevated roles and readmission restores only ordinary membership.
    #[test]
    fn network_control_removal_does_not_restore_prior_elevated_roles() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        initializeTestDeviceProfile(&spaceStore);
        let control =
            NetworkControlStore::new(host.clone()).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        addTestSpaceMember(&spaceStore, "peer-runner");
        control
            .admitMember("peer-runner".to_string())
            .expect("administrator must admit a member");
        control
            .setIdentity(NetworkControlIdentityAssignment {
                nodeId: "peer-runner".to_string(),
                roleId: "runner".to_string(),
            })
            .expect("administrator must grant runner role");
        assert!(control
            .nodeHasCapability("peer-runner", "runtime.execute", None)
            .expect("runner capability must materialize"));

        control
            .removeMember("peer-runner".to_string())
            .expect("administrator must remove the member");
        control
            .admitMember("peer-runner".to_string())
            .expect("administrator must readmit the member");

        assert!(!control
            .nodeHasCapability("peer-runner", "runtime.execute", None)
            .expect("readmitted member must not regain runner capability"));
        assert!(control
            .nodeHasCapability("peer-runner", "network.user", None)
            .expect("readmitted member must receive ordinary capability"));
    }

    /// Verifies identity definitions are immutable and each device has one current identity.
    #[test]
    fn network_control_preserves_identity_definition_and_device_state_invariants() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        initializeTestDeviceProfile(&spaceStore);
        let control =
            NetworkControlStore::new(host.clone()).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        addTestSpaceMember(&spaceStore, "peer-identity");
        control
            .admitMember("peer-identity".to_string())
            .expect("administrator must admit the identity test device");
        control
            .defineRole(NetworkControlRole {
                roleId: "archive".to_string(),
                displayName: "Archive".to_string(),
                capabilities: BTreeSet::from(["storage.provide".to_string()]),
            })
            .expect("administrator must define the archive role");
        assert!(
            control
                .defineRole(NetworkControlRole {
                    roleId: "archive".to_string(),
                    displayName: "Replacement archive".to_string(),
                    capabilities: BTreeSet::from(["storage.provide".to_string()]),
                })
                .is_err(),
            "a role id must have one immutable definition"
        );
        control
            .setIdentity(NetworkControlIdentityAssignment {
                nodeId: "peer-identity".to_string(),
                roleId: "archive".to_string(),
            })
            .expect("administrator must grant the archive role");
        control
            .setIdentity(NetworkControlIdentityAssignment {
                nodeId: "peer-identity".to_string(),
                roleId: "user".to_string(),
            })
            .expect("setting a new identity must replace the device's current identity");
        assert_eq!(
            control
                .currentState()
                .expect("control state must materialize")
                .deviceIdentityIds
                .get("peer-identity")
                .map(String::as_str),
            Some("user"),
            "each device must expose only its current identity"
        );
        control
            .removeMember("peer-identity".to_string())
            .expect("administrator must remove the identity test device");
        assert!(
            control
                .currentState()
                .expect("control state must materialize")
                .deviceIdentityIds
                .get("peer-identity")
                .is_none(),
            "removal must clear the former member identity"
        );
    }

    /// Verifies no command can leave a Space control policy without an administrator.
    #[test]
    fn network_control_rejects_removing_or_revoking_the_last_administrator() {
        let host = Arc::new(MemoryStorageHost::default());
        let spaceStore = CoreSpaceStore::new(host.clone());
        initializeTestDeviceProfile(&spaceStore);
        let administratorNodeId = CoreNodeIdentityStore::new(host.clone())
            .initialize()
            .expect("test node identity must initialize")
            .nodeId;
        let control =
            NetworkControlStore::new(host).expect("network control store must initialize");
        control
            .bootstrapCurrentSpace()
            .expect("Space creator must bootstrap administrator policy");
        assert!(
            control.removeMember(administratorNodeId.clone()).is_err(),
            "the last administrator must not be removable"
        );
        assert!(
            control
                .setIdentity(NetworkControlIdentityAssignment {
                    nodeId: administratorNodeId.clone(),
                    roleId: "user".to_string(),
                })
                .is_err(),
            "the last administrator identity must not be replaceable"
        );
        assert!(
            control.clearIdentity(administratorNodeId).is_err(),
            "the last administrator identity must not be clearable"
        );
    }
}
