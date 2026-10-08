//! Connection discovery is separate from liveness, which belongs to authenticated channels.
use super::*;
use futures_util::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use operit_store::NetworkControlStore::NetworkControlStore;
use tokio::sync::oneshot;

const DISCOVERY_INTERVAL_MS: u64 = 3_000;
const CONNECT_DEADLINE_MS: u64 = 30_000;
const MAX_CONNECTING_PEERS: usize = 4;

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
        let mut worker = self.state.availability.lock().map_err(|_| error("Peer availability lock poisoned"))?;
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
                                if let Some(state) = state.upgrade() {
                                    HostRuntimePeerService { state }.refreshPeerAvailability(&node);
                                }
                                operit_util::AppLogger::AppLogger::trace("RuntimePeerService", &format!("Peer connection attempt failed peer={node} error={error}"));
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
                        // Round-robin admission prevents unreachable peers from monopolizing dial slots.
                        let mut peers = peers;
                        peers.sort_by(|left, right| left.nodeId.cmp(&right.nodeId));
                        let split = peers.partition_point(|peer| lastPeer.as_ref().is_some_and(|last| &peer.nodeId <= last));
                        peers.rotate_left(split);
                        for peer in peers {
                            if service.requirePeerConnectionAllowed(&peer.nodeId).is_err() {
                                service.refreshPeerAvailability(&peer.nodeId);
                                continue;
                            }
                            if connecting.contains(&peer.nodeId) || service.hasLiveChannel(&peer.nodeId) { continue; }
                            if !(peer.outbound || service.spaceOutbound(&peer.nodeId).is_ok()) { continue; }
                            if connecting.len() >= MAX_CONNECTING_PEERS.min(service.state.limits.concurrentProbes) { break; }
                            let node = peer.nodeId;
                            lastPeer = Some(node.clone());
                            connecting.insert(node.clone());
                            let service = service.clone();
                            let tasks = tasks.clone();
                            pending.push(Box::pin(async move {
                                let result = tokio::select! {
                                    result = service.acquirePooledChannel(&node) => result.map(drop),
                                    _ = tasks.waitForHostRuntimeDelay(CONNECT_DEADLINE_MS) => Err(CoreLinkError::new("PEER_CONNECT_TIMEOUT", "Authenticated connection establishment timed out")),
                                };
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
        let worker = self.state.availability.lock().unwrap().take();
        if let Some(AvailabilityWorker { stop, done }) = worker {
            let _ = stop.send(());
            let _ = done.await;
        }
    }
}

/// Established reusable sessions use the channel's own bounded exchange. This
/// keeps correlation, cancellation safety and close-on-timeout in one place.
pub(super) async fn probeConnectedSession(channel: &Channel) -> Result<(), CoreLinkError> {
    if channel.duplex().is_some() {
        let id = nextCoreRouteRequestId("serial-ping");
        match channel.exchange(CoreLinkRequest::Call(CoreCallRequest::new(
            id.clone(), "$peer.keepalive", "ping", CoreValue::Null,
        ))).await? {
            CoreLinkResponse::Call(CoreCallResponse { requestId, result: Ok(CoreValue::Null) }) if requestId.0 == id => {},
            _ => { channel.close().await; return Err(error("Serial keepalive response mismatch")); }
        }
    }
    Ok(())
}
