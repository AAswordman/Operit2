use operit_host_api::TimeUtils::currentTimeMillis;
use operit_store::PreferencesDataStore::StateFlow;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpaceSyncStatus {
    Syncing,
    Waiting,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpaceSyncStage {
    Checking,
    Records,
    Files,
}

/// Local execution progress, independent of membership and replication receipts.
/// `exchangedRecords` counts confirmed exchanges, not materialized chat messages;
/// `transferredBytes` includes retransmissions. Only the current file has a total.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceSyncProgress {
    pub spaceId: String,
    pub peerNodeId: String,
    pub startedAt: i64,
    pub updatedAt: i64,
    pub status: SpaceSyncStatus,
    pub stage: SpaceSyncStage,
    pub exchangedRecords: u64,
    pub transferredBytes: i64,
    pub fileBytes: i64,
    pub fileSize: i64,
    pub retryAt: Option<i64>,
    pub error: Option<String>,
}

pub(crate) struct SyncProgressTracker {
    flow: StateFlow<Vec<SpaceSyncProgress>>,
}

impl Default for SyncProgressTracker {
    fn default() -> Self {
        Self { flow: StateFlow::new(Vec::new()) }
    }
}

impl SyncProgressTracker {
    pub(crate) fn flow(&self) -> StateFlow<Vec<SpaceSyncProgress>> {
        self.flow.clone()
    }

    fn update(&self, change: impl Fn(&mut Vec<SpaceSyncProgress>)) {
        loop {
            let previous = self.flow.value();
            let mut next = previous.clone();
            change(&mut next);
            if next == previous || self.flow.compare_and_set(previous, next) {
                break;
            }
        }
    }

    pub(crate) fn retain(&self, spaceId: &str, peers: &[String]) {
        self.update(|progress| progress.retain(|item| item.spaceId == spaceId && peers.contains(&item.peerNodeId)));
    }

    pub(crate) fn remove(&self, peer: &str) {
        self.update(|progress| progress.retain(|item| item.peerNodeId != peer));
    }

    pub(crate) fn clear(&self) {
        self.flow.set_value(Vec::new());
    }

    pub(crate) fn begin(&self, peer: &str, spaceId: &str) {
        let now = currentTimeMillis();
        self.update(|progress| {
            progress.retain(|item| item.spaceId == spaceId);
            if let Some(item) = progress.iter_mut().find(|item| item.peerNodeId == peer) {
                if item.status != SpaceSyncStatus::Completed {
                    item.status = SpaceSyncStatus::Syncing;
                    item.stage = SpaceSyncStage::Checking;
                    item.updatedAt = now;
                    item.retryAt = None;
                    item.error = None;
                    return;
                }
                progress.retain(|item| item.peerNodeId != peer);
            }
            progress.push(SpaceSyncProgress {
                spaceId: spaceId.into(), peerNodeId: peer.into(), startedAt: now, updatedAt: now,
                status: SpaceSyncStatus::Syncing, stage: SpaceSyncStage::Checking,
                exchangedRecords: 0, transferredBytes: 0, fileBytes: 0, fileSize: 0,
                retryAt: None, error: None,
            });
        });
    }

    fn changePeer(&self, peer: &str, change: impl Fn(&mut SpaceSyncProgress)) {
        self.update(|progress| {
            if let Some(item) = progress.iter_mut().find(|item| item.peerNodeId == peer) {
                change(item);
            }
        });
    }

    pub(crate) fn records(&self, peer: &str, count: usize) {
        self.changePeer(peer, |item| {
            item.stage = SpaceSyncStage::Records;
            item.fileBytes = 0;
            item.fileSize = 0;
            item.exchangedRecords = item.exchangedRecords.saturating_add(count as u64);
            item.updatedAt = currentTimeMillis();
        });
    }

    pub(crate) fn beginFile(&self, peer: &str, size: i64) {
        self.changePeer(peer, |item| {
            item.stage = SpaceSyncStage::Files;
            item.fileBytes = 0;
            item.fileSize = size;
            item.updatedAt = currentTimeMillis();
        });
    }

    /// Chunk traffic can be fast; publish at most five updates per second,
    /// always publishing the final offset. Accumulate bytes since the last emit.
    pub(crate) fn fileOffset(&self, peer: &str, offset: i64) {
        let now = currentTimeMillis();
        self.changePeer(peer, |item| {
            if now - item.updatedAt < 200 && offset != item.fileSize { return; }
            let offset = offset.clamp(item.fileBytes, item.fileSize);
            item.transferredBytes = item.transferredBytes.saturating_add(offset - item.fileBytes);
            item.fileBytes = offset;
            item.updatedAt = now;
        });
    }

    pub(crate) fn finish(&self, peer: &str, pending: bool, retryAt: Option<i64>, error: Option<String>) {
        self.changePeer(peer, |item| {
            item.status = if pending || error.is_some() { SpaceSyncStatus::Waiting } else { SpaceSyncStatus::Completed };
            item.retryAt = retryAt;
            item.error = error.clone();
            item.updatedAt = currentTimeMillis();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_passes_and_retries_keep_one_session_and_confirmed_counts() {
        let progress = SyncProgressTracker::default();
        progress.begin("peer", "space");
        progress.records("peer", 512);
        let started = progress.flow().value()[0].startedAt;
        progress.finish("peer", true, None, None);
        assert_eq!(progress.flow().value()[0].status, SpaceSyncStatus::Waiting);
        progress.begin("peer", "space");
        progress.records("peer", 10);
        progress.finish("peer", false, Some(100), Some("offline".into()));
        progress.begin("peer", "space");
        let item = progress.flow().value().remove(0);
        assert_eq!(item.startedAt, started);
        assert_eq!(item.exchangedRecords, 522);
        assert_eq!(item.error, None);
        progress.finish("peer", false, None, None);
        assert_eq!(progress.flow().value()[0].status, SpaceSyncStatus::Completed);
        progress.begin("peer", "space");
        assert_eq!(progress.flow().value()[0].exchangedRecords, 0);
    }

    #[test]
    fn file_totals_are_per_file_and_retransmission_counts_as_traffic() {
        let progress = SyncProgressTracker::default();
        progress.begin("peer", "space");
        progress.beginFile("peer", 1024);
        progress.fileOffset("peer", 1024);
        progress.beginFile("peer", 1024);
        progress.fileOffset("peer", 1024);
        let item = progress.flow().value().remove(0);
        assert_eq!(item.fileSize, 1024);
        assert_eq!(item.fileBytes, 1024);
        assert_eq!(item.transferredBytes, 2048);
        progress.records("peer", 1);
        assert_eq!(progress.flow().value()[0].fileSize, 0);
    }

    #[test]
    fn progress_cannot_escape_its_space_or_membership() {
        let progress = SyncProgressTracker::default();
        progress.begin("old-peer", "old-space");
        progress.begin("peer", "space");
        assert_eq!(progress.flow().value().len(), 1);
        progress.retain("space", &[]);
        assert!(progress.flow().value().is_empty());
    }
}
