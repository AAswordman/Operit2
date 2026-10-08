//! Local background polling for outgoing Space joins. A pending RPC must not
//! suspend terminal rendering, input, or refreshes for another join request.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::mpsc;
use std::time::Duration;

use futures_util::stream::{FuturesUnordered, Stream};
use futures_util::{FutureExt, StreamExt};
use operit_node_runtime::RuntimeRemoteLinkService::{
    RuntimeRemoteLinkService, SpaceJoinRequest, SpaceJoinStatus,
};

use crate::tui::NetworkUiEvent;

const OUTGOING_JOIN_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn spawn_outgoing_join_monitor(
    network: RuntimeRemoteLinkService,
    events: mpsc::Sender<NetworkUiEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_local(async move {
        let mut interval = tokio::time::interval(OUTGOING_JOIN_REFRESH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let ticks = futures_util::stream::unfold(interval, |mut interval| async move {
            interval.tick().await;
            Some(((), interval))
        })
        .boxed_local();
        let list_network = network.clone();
        monitor_outgoing_joins(
            ticks,
            move || list_network.outgoingDeviceSpaceJoins(),
            move |id| {
                let network = network.clone();
                let events = events.clone();
                async move {
                    let Ok(updated) = network.refreshDeviceSpaceJoin(id).await else {
                        // Preserve offline requests; the next tick retries them.
                        return;
                    };
                    if space_join_is_active(&updated.status) {
                        return;
                    }
                    if events
                        .send(NetworkUiEvent::OutgoingJoinSettled(updated))
                        .is_err()
                    {
                        return;
                    }
                    // Report the decision before fetching snapshots: even a
                    // slow reviewer must not delay the status or block the UI.
                    let _ = events.send(network_snapshot(&network).await);
                }
            },
        )
        .await;
    })
}

pub(super) async fn network_snapshot(network: &RuntimeRemoteLinkService) -> NetworkUiEvent {
    let prompts = network.pairingPrompts().unwrap_or_default();
    let requests = network.incomingDeviceSpaceJoins().await.ok();
    NetworkUiEvent::Snapshot { prompts, requests }
}

/// At most one refresh per request is in flight. Distinct requests are polled
/// concurrently, so a live but unresponsive peer cannot hold up other joins.
/// Dropping/aborting the monitor drops every pending future; ordinary ticks do
/// not cancel RPCs or repeatedly enqueue copies of a stalled request.
async fn monitor_outgoing_joins<Ticks, List, Refresh, RefreshFuture>(
    mut ticks: Ticks,
    mut list: List,
    mut refresh: Refresh,
) where
    Ticks: Stream<Item = ()> + Unpin,
    List: FnMut() -> Result<Vec<SpaceJoinRequest>, String>,
    Refresh: FnMut(String) -> RefreshFuture,
    RefreshFuture: Future<Output = ()> + 'static,
{
    let mut in_flight = BTreeSet::new();
    let mut refreshes = FuturesUnordered::new();
    loop {
        tokio::select! {
            tick = ticks.next() => {
                if tick.is_none() {
                    return;
                }
                let Ok(requests) = list() else {
                    continue;
                };
                for request in requests {
                    if !space_join_is_active(&request.status)
                        || !in_flight.insert(request.requestId.clone())
                    {
                        continue;
                    }
                    let id = request.requestId;
                    let future = refresh(id.clone());
                    refreshes.push(async move {
                        future.await;
                        id
                    }.boxed_local());
                }
            }
            Some(id) = refreshes.next(), if !refreshes.is_empty() => {
                in_flight.remove(&id);
            }
        }
    }
}

/// Mirrors the runtime's active-status rule for outgoing refreshes.
pub(super) fn space_join_is_active(status: &SpaceJoinStatus) -> bool {
    matches!(
        status,
        SpaceJoinStatus::Pending | SpaceJoinStatus::Approving | SpaceJoinStatus::Approved
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::future::pending;
    use std::rc::Rc;

    use futures_util::stream::LocalBoxStream;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

    fn request(id: &str, status: SpaceJoinStatus) -> SpaceJoinRequest {
        SpaceJoinRequest {
            requestId: id.to_string(),
            targetDeviceId: format!("peer-{id}"),
            applicantDeviceId: "local".to_string(),
            applicantName: "local".to_string(),
            spaceName: "space".to_string(),
            status,
            createdAt: 0,
            expiresAt: i64::MAX,
            canApprove: false,
            reviewerDeviceId: None,
            reviewerName: None,
            reviewerHops: None,
            assignmentVersion: 0,
            decisionApprove: None,
        }
    }

    fn ticks(receiver: UnboundedReceiver<()>) -> LocalBoxStream<'static, ()> {
        futures_util::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|tick| (tick, receiver))
        })
        .boxed_local()
    }

    async fn receive<T>(receiver: &mut UnboundedReceiver<T>) -> T {
        tokio::time::timeout(Duration::from_secs(2), receiver.recv())
            .await
            .expect("background monitor must make progress")
            .expect("test event channel must remain open")
    }

    async fn abort(task: tokio::task::JoinHandle<()>) {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn stalled_join_does_not_block_ui_or_another_join() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (finished_sender, mut finished_receiver) = unbounded_channel();
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    || {
                        Ok(vec![
                            request("slow", SpaceJoinStatus::Pending),
                            request("fast", SpaceJoinStatus::Pending),
                        ])
                    },
                    move |id| {
                        let finished_sender = finished_sender.clone();
                        async move {
                            if id == "slow" {
                                pending::<()>().await;
                            }
                            finished_sender.send(id).unwrap();
                        }
                    },
                ));
                tick_sender.send(()).unwrap();
                assert_eq!(receive(&mut finished_receiver).await, "fast");
                // Use an Rc to exercise the actual non-Send local executor.
                // A terminal frame/input task must run while the slow RPC waits.
                let frames = Rc::new(Cell::new(0));
                let ui_frames = frames.clone();
                tokio::time::timeout(Duration::from_secs(2), async move {
                    tokio::task::spawn_local(async move { ui_frames.set(1) })
                        .await
                        .unwrap();
                })
                .await
                .expect("UI must not wait for the slow join");
                assert_eq!(frames.get(), 1);
                abort(task).await;
            })
            .await;
    }

    struct DropFlag(Rc<Cell<bool>>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[tokio::test]
    async fn repeated_ticks_do_not_duplicate_a_stalled_join_and_abort_drops_it() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (listed_sender, mut listed_receiver) = unbounded_channel();
                let (started_sender, mut started_receiver) = unbounded_channel();
                let dropped = Rc::new(Cell::new(false));
                let future_dropped = dropped.clone();
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    move || {
                        listed_sender.send(()).unwrap();
                        Ok(vec![request("slow", SpaceJoinStatus::Pending)])
                    },
                    move |id| {
                        let guard = DropFlag(future_dropped.clone());
                        started_sender.send(id).unwrap();
                        async move {
                            let _guard = guard;
                            pending::<()>().await;
                        }
                    },
                ));
                tick_sender.send(()).unwrap();
                receive(&mut listed_receiver).await;
                assert_eq!(receive(&mut started_receiver).await, "slow");
                for _ in 0..3 {
                    tick_sender.send(()).unwrap();
                    receive(&mut listed_receiver).await;
                }
                assert!(started_receiver.try_recv().is_err());
                assert!(!dropped.get());
                abort(task).await;
                assert!(dropped.get());
            })
            .await;
    }

    #[tokio::test]
    async fn completed_attempt_is_retried_if_the_request_remains_active() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (attempt_sender, mut attempt_receiver) = unbounded_channel();
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    || Ok(vec![request("offline", SpaceJoinStatus::Pending)]),
                    move |id| {
                        let attempt_sender = attempt_sender.clone();
                        async move {
                            // Production errors return without changing the
                            // persisted Pending status; a later tick retries.
                            attempt_sender.send(id).unwrap();
                        }
                    },
                ));
                for _ in 0..2 {
                    tick_sender.send(()).unwrap();
                    assert_eq!(receive(&mut attempt_receiver).await, "offline");
                }
                abort(task).await;
            })
            .await;
    }

    #[tokio::test]
    async fn only_active_requests_are_refreshed_and_terminal_requests_stop() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let requests = Rc::new(RefCell::new(vec![
                    request("pending", SpaceJoinStatus::Pending),
                    request("approving", SpaceJoinStatus::Approving),
                    request("approved", SpaceJoinStatus::Approved),
                    request("joined", SpaceJoinStatus::Joined),
                    request("rejected", SpaceJoinStatus::Rejected),
                    request("expired", SpaceJoinStatus::Expired),
                    request("cancelled", SpaceJoinStatus::Cancelled),
                ]));
                let list_requests = requests.clone();
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (listed_sender, mut listed_receiver) = unbounded_channel();
                let (attempt_sender, mut attempt_receiver) = unbounded_channel();
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    move || {
                        listed_sender.send(()).unwrap();
                        Ok(list_requests.borrow().clone())
                    },
                    move |id| {
                        let attempt_sender = attempt_sender.clone();
                        async move { attempt_sender.send(id).unwrap() }
                    },
                ));
                tick_sender.send(()).unwrap();
                receive(&mut listed_receiver).await;
                let mut refreshed = BTreeSet::new();
                for _ in 0..3 {
                    refreshed.insert(receive(&mut attempt_receiver).await);
                }
                assert_eq!(
                    refreshed,
                    ["pending", "approving", "approved"]
                        .map(str::to_string)
                        .into_iter()
                        .collect()
                );
                for request in requests.borrow_mut().iter_mut() {
                    request.status = SpaceJoinStatus::Joined;
                }
                tick_sender.send(()).unwrap();
                receive(&mut listed_receiver).await;
                assert!(attempt_receiver.try_recv().is_err());
                abort(task).await;
            })
            .await;
    }

    #[tokio::test]
    async fn failed_snapshot_is_retried_on_the_next_tick() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (listed_sender, mut listed_receiver) = unbounded_channel();
                let (attempt_sender, mut attempt_receiver) = unbounded_channel();
                let mut fail_once = true;
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    move || {
                        listed_sender.send(()).unwrap();
                        if std::mem::take(&mut fail_once) {
                            Err("temporary store failure".to_string())
                        } else {
                            Ok(vec![request("join", SpaceJoinStatus::Pending)])
                        }
                    },
                    move |id| {
                        let attempt_sender = attempt_sender.clone();
                        async move { attempt_sender.send(id).unwrap() }
                    },
                ));
                tick_sender.send(()).unwrap();
                receive(&mut listed_receiver).await;
                assert!(attempt_receiver.try_recv().is_err());
                tick_sender.send(()).unwrap();
                assert_eq!(receive(&mut attempt_receiver).await, "join");
                abort(task).await;
            })
            .await;
    }

    #[tokio::test]
    async fn closing_the_monitor_drops_pending_refreshes() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let (tick_sender, tick_receiver) = unbounded_channel();
                let (started_sender, mut started_receiver) = unbounded_channel();
                let dropped = Rc::new(Cell::new(false));
                let future_dropped = dropped.clone();
                let task = tokio::task::spawn_local(monitor_outgoing_joins(
                    ticks(tick_receiver),
                    || Ok(vec![request("slow", SpaceJoinStatus::Pending)]),
                    move |_| {
                        let guard = DropFlag(future_dropped.clone());
                        started_sender.send(()).unwrap();
                        async move {
                            let _guard = guard;
                            pending::<()>().await;
                        }
                    },
                ));
                tick_sender.send(()).unwrap();
                receive(&mut started_receiver).await;
                drop(tick_sender);
                tokio::time::timeout(Duration::from_secs(2), task)
                    .await
                    .expect("monitor must stop without waiting for its RPCs")
                    .unwrap();
                assert!(dropped.get());
            })
            .await;
    }
}
