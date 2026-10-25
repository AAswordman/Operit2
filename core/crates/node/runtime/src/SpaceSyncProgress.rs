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

const LARGE_SYNC_DELAY_MS: i64 = 3_000;
const LARGE_SYNC_RECORDS: u64 = 256;
const LARGE_SYNC_BYTES: i64 = 1024 * 1024;

impl SpaceSyncProgress {
    fn hasSubstantialWork(&self) -> bool {
        self.exchangedRecords >= LARGE_SYNC_RECORDS
            || self.transferredBytes >= LARGE_SYNC_BYTES
            || self.fileSize >= LARGE_SYNC_BYTES
    }
}

#[derive(Clone, PartialEq)]
struct SyncProgressSession {
    progress: SpaceSyncProgress,
    visible: bool,
}

pub(crate) struct SyncProgressTracker {
    sessions: StateFlow<Vec<SyncProgressSession>>,
}

impl Default for SyncProgressTracker {
    fn default() -> Self {
        Self {
            sessions: StateFlow::new(Vec::new()),
        }
    }
}

impl SyncProgressTracker {
    /// Hidden sessions never cross the proxy or rebuild the UI.
    pub(crate) fn flow(&self) -> StateFlow<Vec<SpaceSyncProgress>> {
        self.sessions.map(|sessions| {
            sessions
                .into_iter()
                .filter(|session| session.visible)
                .map(|session| session.progress)
                .collect()
        })
    }

    fn update(&self, change: impl Fn(&mut Vec<SyncProgressSession>)) {
        loop {
            let previous = self.sessions.value();
            let mut next = previous.clone();
            change(&mut next);
            if next == previous || self.sessions.compare_and_set(previous, next) {
                break;
            }
        }
    }

    pub(crate) fn retain(&self, spaceId: &str, peers: &[String]) {
        self.update(|sessions| {
            sessions.retain(|session| {
                session.progress.spaceId == spaceId && peers.contains(&session.progress.peerNodeId)
            })
        });
    }

    pub(crate) fn remove(&self, peer: &str) {
        self.update(|sessions| sessions.retain(|session| session.progress.peerNodeId != peer));
    }

    pub(crate) fn clear(&self) {
        self.sessions.set_value(Vec::new());
    }

    pub(crate) fn begin(&self, peer: &str, spaceId: &str) {
        let now = currentTimeMillis();
        self.update(|sessions| {
            sessions.retain(|session| {
                session.progress.spaceId == spaceId
                    && (session.progress.status != SpaceSyncStatus::Completed
                        || now - session.progress.updatedAt < LARGE_SYNC_DELAY_MS)
            });
            if let Some(session) = sessions.iter_mut().find(|session| {
                session.progress.peerNodeId == peer
                    && session.progress.status != SpaceSyncStatus::Completed
            }) {
                let item = &mut session.progress;
                item.status = SpaceSyncStatus::Syncing;
                item.stage = SpaceSyncStage::Checking;
                item.updatedAt = now;
                item.retryAt = None;
                item.error = None;
                return;
            }
            sessions.push(SyncProgressSession {
                visible: false,
                progress: SpaceSyncProgress {
                    spaceId: spaceId.into(),
                    peerNodeId: peer.into(),
                    startedAt: now,
                    updatedAt: now,
                    status: SpaceSyncStatus::Syncing,
                    stage: SpaceSyncStage::Checking,
                    exchangedRecords: 0,
                    transferredBytes: 0,
                    fileBytes: 0,
                    fileSize: 0,
                    retryAt: None,
                    error: None,
                },
            });
        });
    }

    pub(crate) fn revealDelayMs(&self, peer: &str) -> Option<u64> {
        self.sessions
            .value()
            .iter()
            .find(|session| {
                session.progress.peerNodeId == peer
                    && session.progress.status == SpaceSyncStatus::Syncing
                    && !session.visible
            })
            .map(|session| {
                (LARGE_SYNC_DELAY_MS - (currentTimeMillis() - session.progress.startedAt))
                    .clamp(0, LARGE_SYNC_DELAY_MS) as u64
            })
    }

    pub(crate) fn reveal(&self, peer: &str) {
        self.update(|sessions| {
            for session in sessions.iter_mut().filter(|session| {
                session.progress.peerNodeId == peer
                    && session.progress.status != SpaceSyncStatus::Completed
            }) {
                session.visible |= session.progress.hasSubstantialWork();
            }
        });
    }

    fn changePeer(&self, peer: &str, change: impl Fn(&mut SpaceSyncProgress)) {
        self.update(|sessions| {
            if let Some(session) = sessions.iter_mut().find(|session| {
                session.progress.peerNodeId == peer
                    && session.progress.status != SpaceSyncStatus::Completed
            }) {
                change(&mut session.progress);
                // A pass can cross the volume threshold after its delay fired.
                session.visible |= session.progress.hasSubstantialWork()
                    && currentTimeMillis() - session.progress.startedAt >= LARGE_SYNC_DELAY_MS;
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
    pub(crate) fn fileOffset(&self, peer: &str, offset: i64, flush: bool) {
        let now = currentTimeMillis();
        self.changePeer(peer, |item| {
            if !flush && now - item.updatedAt < 200 && offset != item.fileSize {
                return;
            }
            let offset = offset.clamp(item.fileBytes, item.fileSize);
            item.transferredBytes = item
                .transferredBytes
                .saturating_add(offset - item.fileBytes);
            item.fileBytes = offset;
            item.updatedAt = now;
        });
    }

    pub(crate) fn finish(
        &self,
        peer: &str,
        pending: bool,
        retryAt: Option<i64>,
        error: Option<String>,
    ) {
        self.changePeer(peer, |item| {
            item.status = if pending || error.is_some() {
                SpaceSyncStatus::Waiting
            } else {
                SpaceSyncStatus::Completed
            };
            item.retryAt = retryAt;
            item.error = error.clone();
            item.updatedAt = currentTimeMillis();
        });
        // Fast successful exchanges have no observable lifecycle at all.
        self.update(|sessions| {
            sessions.retain(|session| {
                session.visible || session.progress.status != SpaceSyncStatus::Completed
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_exchanges_emit_no_progress_events() {
        let progress = SyncProgressTracker::default();
        let flow = progress.flow();
        let events = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = events.clone();
        flow.subscribe(move |_| {
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        for _ in 0..20 {
            progress.begin("peer", "space");
            progress.records("peer", 1);
            progress.finish("peer", false, None, None);
        }
        assert!(flow.value().is_empty());
        assert_eq!(events.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn slow_small_work_stays_silent_but_later_backlog_reveals() {
        let progress = SyncProgressTracker::default();
        progress.begin("peer", "space");
        progress.update(|sessions| sessions[0].progress.startedAt -= LARGE_SYNC_DELAY_MS + 1);
        progress.reveal("peer");
        progress.records("peer", 2);
        progress.beginFile("peer", 6);
        progress.fileOffset("peer", 6, true);
        assert!(progress.flow().value().is_empty());
        progress.finish("peer", false, None, None);
        assert!(progress.flow().value().is_empty());

        progress.begin("peer", "space");
        progress.update(|sessions| sessions[0].progress.startedAt -= LARGE_SYNC_DELAY_MS + 1);
        progress.reveal("peer");
        progress.records("peer", 256);
        assert_eq!(progress.flow().value().len(), 1);
    }

    #[test]
    fn pending_passes_and_retries_keep_one_session_and_confirmed_counts() {
        let progress = SyncProgressTracker::default();
        progress.begin("peer", "space");
        progress.records("peer", 512);
        progress.reveal("peer");
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
        assert_eq!(
            progress.flow().value()[0].status,
            SpaceSyncStatus::Completed
        );
        progress.begin("peer", "space");
        assert_eq!(
            progress
                .sessions
                .value()
                .last()
                .unwrap()
                .progress
                .exchangedRecords,
            0
        );
        assert!(!progress.sessions.value().last().unwrap().visible);
    }

    #[test]
    fn file_totals_are_per_file_and_retransmission_counts_as_traffic() {
        let progress = SyncProgressTracker::default();
        progress.begin("peer", "space");
        progress.records("peer", 256);
        progress.reveal("peer");
        progress.beginFile("peer", 1024);
        progress.fileOffset("peer", 1024, false);
        progress.beginFile("peer", 1024);
        progress.fileOffset("peer", 1024, false);
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
        progress.records("peer", 256);
        progress.reveal("peer");
        assert_eq!(progress.flow().value().len(), 1);
        progress.retain("space", &[]);
        assert!(progress.flow().value().is_empty());
    }
}
