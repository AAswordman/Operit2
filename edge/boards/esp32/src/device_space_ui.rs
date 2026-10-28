#![allow(non_snake_case)]
//! Shared firmware/simulator controller. UI state is bounded and volatile;
//! identity, pairing and consent remain in the existing node services.
use operit_host_api::{HostRuntimeAsyncTask, HostRuntimeTaskSchedulerHost};
use operit_edge_sdk::runtime::{
    NodeServices::{DiscoveredPeer, NodeServices, PeerEndpoint, PeerTransport, PendingPairing},
    NodeSpaceService::{NodeSpaceService, SpaceJoinRequest, SpaceJoinStatus},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    epoch: u64,
    // 0 overview, 1 discovery, 2 candidates, 3 code entry, 4 busy.
    phase: u8,
    status: String,
    candidates: Vec<DiscoveredPeer>,
    pairing: Option<PendingPairing>,
    request: Option<SpaceJoinRequest>,
    joinTarget: Option<String>,
    polling: bool,
}
fn active(request: &SpaceJoinRequest) -> bool {
    matches!(
        request.status,
        SpaceJoinStatus::Pending | SpaceJoinStatus::Approving | SpaceJoinStatus::Approved
    )
}
fn showRequest(state: &mut State, request: SpaceJoinRequest) {
    state.phase = 0;
    state.status = match request.status {
        SpaceJoinStatus::Joined => "已加入设备空间".into(),
        SpaceJoinStatus::Pending => format!(
            "等待{}审批加入「{}」",
            request.reviewerName.as_deref().unwrap_or("空间管理员"),
            request.spaceName
        ),
        SpaceJoinStatus::Approving | SpaceJoinStatus::Approved => {
            "审批已通过，正在同步设备空间…".into()
        }
        SpaceJoinStatus::Rejected => "加入申请被拒绝，可重新申请".into(),
        SpaceJoinStatus::Cancelled => "加入申请已取消".into(),
        SpaceJoinStatus::Expired => "加入申请已过期，可重新申请".into(),
    };
    state.status = state.status.chars().take(100).collect();
    state.joinTarget = Some(request.targetDeviceId.clone());
    state.request = Some(request);
}
fn transport(address: &str) -> PeerTransport {
    if address.starts_with("http://") || address.starts_with("https://") {
        PeerTransport::Http
    } else if address.starts_with("ws://") || address.starts_with("wss://") {
        PeerTransport::WebSocket
    } else {
        PeerTransport::Tcp
    }
}

pub struct DeviceSpaceUi {
    services: NodeServices,
    space: Arc<NodeSpaceService>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    state: Arc<Mutex<State>>,
    nextPoll: Instant,
}
impl DeviceSpaceUi {
    pub fn new(
        services: NodeServices,
        space: Arc<NodeSpaceService>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    ) -> Self {
        Self {
            services,
            space,
            scheduler,
            state: Arc::new(Mutex::new(State::default())),
            nextPoll: Instant::now(),
        }
    }
    pub fn isBusy(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.polling || state.phase == 4
    }

    pub fn action(&mut self, action: &str) -> bool {
        match action {
            "edge_pair_discover" => self.discover(),
            "edge_pair_cancel" => self.cancel(),
            "edge_space_join" => self.join(),
            _ => {
                if let Some(index) = action
                    .strip_prefix("edge_pair_candidate:")
                    .and_then(|v| v.parse().ok())
                {
                    self.start(index);
                } else if let Some(code) = action.strip_prefix("edge_pair_submit:") {
                    self.finish(code);
                } else {
                    return false;
                }
            }
        }
        true
    }
    fn schedule(&self, name: &str, epoch: u64, fallback: u8, task: HostRuntimeAsyncTask) {
        let guarded: HostRuntimeAsyncTask = Box::new(move || Box::pin(async move {
            let _work = crate::space_work::acquire().await;
            task().await;
        }));
        if let Err(error) = self.scheduler.scheduleHostRuntimeBlockingAsyncTask(name, guarded) {
            let mut state = self.state.lock().unwrap();
            if state.epoch == epoch {
                state.phase = fallback;
                state.polling = false;
                state.status = error.message.chars().take(100).collect();
            }
        }
    }
    fn discover(&self) {
        let epoch = {
            let mut state = self.state.lock().unwrap();
            if state.polling
                || !matches!(state.phase, 0 | 2)
                || state.request.as_ref().is_some_and(active)
            {
                return;
            }
            state.epoch = state.epoch.wrapping_add(1);
            state.phase = 1;
            state.status = "正在发现附近节点…".into();
            state.candidates.clear();
            state.epoch
        };
        let services = self.services.peers();
        let state = self.state.clone();
        self.schedule(
            "edge-space-discover",
            epoch,
            0,
            Box::new(move || {
                Box::pin(async move {
                    let result = services.discoverPeers(1800).await;
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    state.phase = 2;
                    match result {
                        Ok(mut peers) => {
                            peers.truncate(3);
                            state.status = if peers.is_empty() {
                                "未发现节点，请确认同一局域网及 Core 监听".into()
                            } else {
                                "选择节点，再输入其显示的配对码".into()
                            };
                            state.candidates = peers;
                        }
                        Err(error) => {
                            state.status = error.message.chars().take(100).collect();
                        }
                    }
                })
            }),
        );
    }
    fn start(&self, index: usize) {
        let (epoch, peer) = {
            let mut state = self.state.lock().unwrap();
            if state.phase != 2 || state.polling {
                return;
            }
            let Some(peer) = state.candidates.get(index).cloned() else {
                return;
            };
            state.epoch = state.epoch.wrapping_add(1);
            state.phase = 4;
            state.status = "正在请求配对…".into();
            (state.epoch, peer)
        };
        let services = self.services.peers();
        let state = self.state.clone();
        self.schedule(
            "edge-space-pair-start",
            epoch,
            2,
            Box::new(move || {
                Box::pin(async move {
                    let mode = transport(&peer.address);
                    let result = services
                        .startPairing(
                            PeerEndpoint {
                                nodeId: peer.nodeId,
                                address: peer.address,
                            },
                            Some(mode),
                        )
                        .await;
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    match result {
                        Ok(pairing) => {
                            state.status = "请输入对端显示的六位配对码".into();
                            state.pairing = Some(pairing);
                            state.phase = 3;
                        }
                        Err(error) => {
                            state.status = error.message.chars().take(100).collect();
                            state.phase = 2;
                        }
                    }
                })
            }),
        );
    }
    fn finish(&self, code: &str) {
        let (epoch, pairing) = {
            let mut state = self.state.lock().unwrap();
            if state.phase != 3 {
                return;
            }
            if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
                state.status = "请输入六位数字配对码".into();
                return;
            }
            let Some(pairing) = state.pairing.clone() else {
                return;
            };
            state.epoch = state.epoch.wrapping_add(1);
            state.phase = 4;
            state.status = "正在验证配对码…".into();
            (state.epoch, pairing)
        };
        let code = code.to_string();
        let services = self.services.peers();
        let space = self.space.clone();
        let state = self.state.clone();
        self.schedule(
            "edge-space-pair-finish",
            epoch,
            3,
            Box::new(move || {
                Box::pin(async move {
                    let paired = services.finishPairing(&pairing.pairingId, &code).await;
                    if let Err(error) = paired {
                        let mut state = state.lock().unwrap();
                        if state.epoch != epoch {
                            return;
                        }
                        state.phase = 3;
                        state.status = error.message.chars().take(100).collect();
                        return; // Retain the transaction for retry, respecting service attempt/expiry limits.
                    }
                    {
                        let mut state = state.lock().unwrap();
                        if state.epoch != epoch {
                            return;
                        }
                        state.pairing = None;
                        state.candidates.clear();
                        state.joinTarget = Some(pairing.peerNodeId.clone());
                        state.status = "配对完成，正在申请加入设备空间…".into();
                    }
                    let request = space.requestDeviceSpaceJoin(pairing.peerNodeId).await;
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    match request {
                        Ok(request) => showRequest(&mut state, request),
                        Err(error) => {
                            state.phase = 0;
                            state.status = format!(
                                "已配对，加入失败：{}",
                                error.chars().take(70).collect::<String>()
                            );
                        }
                    }
                })
            }),
        );
    }
    fn join(&self) {
        let target = {
            let state = self.state.lock().unwrap();
            if state.phase != 0 || state.polling || state.request.as_ref().is_some_and(active) {
                return;
            }
            state.joinTarget.clone()
        }
        .or_else(|| {
            self.services
                .peers()
                .pairedPeers()
                .ok()?
                .into_iter()
                .find(|peer| peer.outbound)
                .map(|peer| peer.nodeId)
        });
        let Some(target) = target else {
            self.state.lock().unwrap().status = "请先发起配对，或在 Core 上申请加入本机空间".into();
            return;
        };
        let epoch = {
            let mut state = self.state.lock().unwrap();
            state.epoch = state.epoch.wrapping_add(1);
            state.phase = 4;
            state.status = "正在申请加入设备空间…".into();
            state.epoch
        };
        let space = self.space.clone();
        let state = self.state.clone();
        self.schedule(
            "edge-space-join",
            epoch,
            0,
            Box::new(move || {
                Box::pin(async move {
                    let result = space.requestDeviceSpaceJoin(target).await;
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    match result {
                        Ok(request) => showRequest(&mut state, request),
                        Err(error) => {
                            state.phase = 0;
                            state.status = error.chars().take(100).collect();
                        }
                    }
                })
            }),
        );
    }
    fn cancel(&self) {
        let (epoch, pairing, request) = {
            let mut state = self.state.lock().unwrap();
            // Completing confirmation/consent is a mutation. Do not race it with cancel or discovery.
            if state.phase == 4 || state.polling {
                return;
            }
            state.epoch = state.epoch.wrapping_add(1);
            let request = state.request.clone().filter(active);
            state.phase = if state.pairing.is_some() || request.is_some() {
                4
            } else {
                0
            };
            state.candidates.clear();
            state.status = "已取消配对流程".into();
            (state.epoch, state.pairing.clone(), request)
        };
        if pairing.is_none() && request.is_none() {
            return;
        }
        let services = self.services.peers();
        let space = self.space.clone();
        let state = self.state.clone();
        self.schedule(
            "edge-space-cancel",
            epoch,
            if pairing.is_some() { 3 } else { 0 },
            Box::new(move || {
                Box::pin(async move {
                    let result = async {
                        if let Some(pairing) = pairing {
                            services
                                .cancelPairing(&pairing.pairingId)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        if let Some(request) = request {
                            return space
                                .cancelDeviceSpaceJoin(request.requestId)
                                .await
                                .map(Some);
                        }
                        Ok(None)
                    }
                    .await;
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    match result {
                        Ok(Some(request)) => {
                            state.pairing = None;
                            showRequest(&mut state, request);
                        }
                        Ok(None) => {
                            state.pairing = None;
                            state.phase = 0;
                        }
                        Err(error) => {
                            state.phase = if state.pairing.is_some() { 3 } else { 0 };
                            state.status = error.chars().take(100).collect();
                        }
                    }
                })
            }),
        );
    }
    /// Refresh ordinary persisted outgoing consent; never infer admission from pairing.
    pub fn poll(&mut self) {
        if Instant::now() < self.nextPoll {
            return;
        }
        self.nextPoll = Instant::now() + Duration::from_secs(3);
        let epoch = {
            let mut state = self.state.lock().unwrap();
            if state.phase != 0 || state.polling {
                return;
            }
            state.polling = true;
            state.epoch
        };
        let space = self.space.clone();
        let state = self.state.clone();
        self.schedule(
            "edge-space-refresh",
            epoch,
            0,
            Box::new(move || {
                Box::pin(async move {
                    // Recover a pending request after firmware/simulator restart from the shared store.
                    let result = match space.outgoingDeviceSpaceJoins() {
                        Ok(requests) => {
                            let previous = state.lock().unwrap().request.clone().filter(active);
                            let request = requests
                                .iter()
                                .find(|request| active(request))
                                .or_else(|| {
                                    previous.as_ref().and_then(|old| {
                                        requests.iter().find(|r| r.requestId == old.requestId)
                                    })
                                })
                                .cloned();
                            if let Some(request) = request {
                                if active(&request) {
                                    {
                                        let mut state = state.lock().unwrap();
                                        if state.epoch == epoch {
                                            showRequest(&mut state, request.clone());
                                        }
                                    }
                                    Some(space.refreshDeviceSpaceJoin(request.requestId).await)
                                } else {
                                    Some(Ok(request))
                                }
                            } else {
                                None
                            }
                        }
                        Err(error) => Some(Err(error)),
                    };
                    let mut state = state.lock().unwrap();
                    if state.epoch != epoch {
                        return;
                    }
                    state.polling = false;
                    match result {
                        Some(Ok(request)) => showRequest(&mut state, request),
                        Some(Err(error)) => {
                            state.status = format!(
                                "同步未完成：{}",
                                error.chars().take(70).collect::<String>()
                            );
                        }
                        None => {
                            if state.request.as_ref().is_some_and(active) {
                                state.request = None;
                                state.joinTarget = None;
                                state.status = "加入申请已结束或本机配对已解除".into();
                            }
                        }
                    }
                })
            }),
        );
    }
    /// Local unpair/Space exit already went through the shared service. Drop
    /// presentation state so its previous success cannot be shown as current.
    pub fn reset(&mut self) {
        let mut state = self.state.lock().unwrap();
        let epoch = state.epoch.wrapping_add(1);
        *state = State {
            epoch,
            ..State::default()
        };
        self.nextPoll = Instant::now();
    }
    pub fn snapshot(&self) -> serde_json::Value {
        let state = self.state.lock().unwrap();
        serde_json::json!({"phase": state.phase, "status": state.status,
            "pendingJoin": state.request.as_ref().is_some_and(active),
            "candidateIds": state.candidates.iter().map(|peer| &peer.nodeId).collect::<Vec<_>>(),
            "candidates": state.candidates.iter().map(|peer| peer.displayName.chars().take(36).collect::<String>()).collect::<Vec<_>>()})
    }
}
