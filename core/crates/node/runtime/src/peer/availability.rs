//! Connection discovery is separate from liveness, which belongs to authenticated channels.
use super::*;
use futures_util::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use operit_store::NetworkControlStore::NetworkControlStore;
use tokio::sync::oneshot;

const DISCOVERY_INTERVAL_MS: u64 = 3_000;
const MAX_CONNECTING_PEERS: usize = 4;
/// Policy rejections (`PEER_CONNECTION_REVOKED`) are deterministic until an admin
/// admits the peer again, so probing them at the discovery interval is pure waste.
const POLICY_REVOKED_BACKOFF_MS: u64 = 60_000;

pub(super) struct AvailabilityWorker {
    stop: oneshot::Sender<()>,
    done: oneshot::Receiver<()>,
}

impl HostRuntimePeerService {
    /// Rejects transport use forbidden by the current shared Space policy.
    pub(super) fn requirePeerConnectionAllowed(&self, node: &str) -> Result<(), CoreLinkError> {
        let storage = self.state.host.runtimeStorageHost.clone()
            .ok_or_else(|| error("Runtime storage Host is not installed"))?;
        if NetworkControlStore::new(storage).map_err(error)?.nodeIsDisconnected(node).map_err(error)? {
            return Err(CoreLinkError::new("PEER_CONNECTION_REVOKED", "Peer is disconnected by current Space policy"));
        }
        Ok(())
    }

    /// Starts bounded, independent connection attempts; established channels monitor themselves.
    pub(super) fn startAvailabilityWorker(&self) -> Result<(), CoreLinkError> {
        let mut worker = self.state.availability.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if worker.is_some() { return Ok(()); }
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let tasks = scheduler.clone();
        let state = Arc::downgrade(&self.state);
        let (stop, mut stopped) = oneshot::channel();
        let (done, finished) = oneshot::channel();
        scheduler.scheduleHostRuntimeAsyncTask("peer-availability", Box::new(move || Box::pin(async move {
            let mut pending = FuturesUnordered::<LocalBoxFuture<'static, (String, Result<(), CoreLinkError>)>>::new();
            let mut connecting = BTreeSet::new();
            let mut policyRevokedUntil = BTreeMap::<String, std::time::Instant>::new();
            let mut tick = tasks.waitForHostRuntimeDelay(0);
            let mut lastPeer = None;
            loop {
                tokio::select! {
                    biased;
                    _ = &mut stopped => break,
                    result = pending.next(), if !pending.is_empty() => {
                        if let Some((node, result)) = result {
                            connecting.remove(&node);
                            if let Err(error) = result {
                                if error.code == "PEER_CONNECTION_REVOKED" {
                                    policyRevokedUntil.insert(node.clone(), std::time::Instant::now() + std::time::Duration::from_millis(POLICY_REVOKED_BACKOFF_MS));
                                }
                                if let Some(state) = state.upgrade() {
                                    HostRuntimePeerService { state }.refreshPeerAvailability(&node);
                                }
                                operit_util::AppLogger::AppLogger::v_with_level("RuntimePeerService", &format!("Peer connection attempt failed peer={node} error={error}"), operit_util::AppLogger::VERBOSE_LEVEL_3);
                            }
                        }
                    }
                    _ = &mut tick => {
                        tick = tasks.waitForHostRuntimeDelay(DISCOVERY_INTERVAL_MS);
                        let Some(state) = state.upgrade() else { break; };
                        let service = HostRuntimePeerService { state };
                        let peers = match service.pairedPeers() {
                            Ok(peers) => peers,
                            Err(error) => {
                                operit_util::AppLogger::AppLogger::w("RuntimePeerService", &error.to_string());
                                continue;
                            }
                        };
                        let now = std::time::Instant::now();
                        policyRevokedUntil.retain(|_, until| *until > now);
                        // Round-robin admission prevents unreachable peers from monopolizing dial slots.
                        let mut peers = peers;
                        peers.sort_by(|left, right| left.nodeId.cmp(&right.nodeId));
                        let split = peers.partition_point(|peer| lastPeer.as_ref().is_some_and(|last| &peer.nodeId <= last));
                        peers.rotate_left(split);
                        for peer in peers {
                            if policyRevokedUntil.contains_key(&peer.nodeId) { continue; }
                            if service.requirePeerConnectionAllowed(&peer.nodeId).is_err() {
                                service.refreshPeerAvailability(&peer.nodeId);
                                continue;
                            }
                            if connecting.contains(&peer.nodeId) || service.hasLiveChannel(&peer.nodeId) { continue; }
                            if !peer.outbound { continue; }
                            if connecting.len() >= MAX_CONNECTING_PEERS.min(service.state.limits.concurrentProbes) { break; }
                            let node = peer.nodeId;
                            lastPeer = Some(node.clone());
                            connecting.insert(node.clone());
                            let service = service.clone();
                            pending.push(Box::pin(async move {
                                // The acquisition carries its own connect deadline.
                                let result = crate::fault_isolation::link(
                                    &format!("connect peer={node}"), service.acquireChannel(&node),
                                ).await.map(drop);
                                (node, result)
                            }));
                        }
                    }
                }
            }
            drop(pending);
            let _ = done.send(());
        }))).map_err(|e| error(e.to_string()))?;
        *worker = Some(AvailabilityWorker { stop, done: finished });
        Ok(())
    }

    /// Cancels connection establishment and waits without retaining the worker state lock.
    pub(super) async fn stopAvailabilityWorker(&self) {
        let worker = self.state.availability.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        if let Some(AvailabilityWorker { stop, done }) = worker {
            let _ = stop.send(());
            let _ = done.await;
        }
    }
}
