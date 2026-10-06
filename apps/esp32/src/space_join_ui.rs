#![allow(non_snake_case)]
//! UI stays on the main task; shared admission work stays on the Host worker.
use crate::ui::Esp32Ui;
use operit_host_api::{HostResult, HostRuntimeTaskSchedulerHost};
use operit_node_edge::PeerRouter::EdgePeerRouter;
use operit_node_runtime::NodeSpaceService::{NodeSpaceService, SpaceJoinRequest};
use operit_store::CoreSpaceStore::CoreSpaceDeviceProfile;
use std::sync::{
    mpsc::{self, Receiver, SyncSender},
    Arc,
};
use std::time::{Duration, Instant};

enum Event {
    Ready(Result<(), String>),
    Poll(Result<Option<SpaceJoinRequest>, String>),
    Decision(Result<SpaceJoinRequest, String>),
    Left(Result<(), String>),
}
pub struct SpaceJoinUi {
    service: Arc<NodeSpaceService>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    sender: SyncSender<Event>,
    receiver: Receiver<Event>,
    pending: Option<SpaceJoinRequest>,
    ready: bool,
    busy: bool,
    leaveRequested: bool,
    nextPoll: Instant,
    initialization: Option<(Arc<EdgePeerRouter>, CoreSpaceDeviceProfile)>,
}
impl SpaceJoinUi {
    pub fn new(
        service: Arc<NodeSpaceService>,
        router: Arc<EdgePeerRouter>,
        profile: CoreSpaceDeviceProfile,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    ) -> HostResult<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let initialization = Some((router.clone(), profile.clone()));
        let initService = service.clone();
        let result = sender.clone();
        scheduler.scheduleHostRuntimeTask(
            "edge-space-init",
            Box::new(move || {
                let initialized = initService
                    .initialize(profile)
                    .and_then(|_| router.installSpace(initService));
                let _ = result.try_send(Event::Ready(initialized));
            }),
        )?;
        Ok(Self {
            service,
            scheduler,
            sender,
            receiver,
            pending: None,
            ready: false,
            busy: true,
            leaveRequested: false,
            nextPoll: Instant::now(),
            initialization,
        })
    }
    pub fn action(&mut self, action: &str, ui: &mut Esp32Ui) -> bool {
        if action == "edge_space_leave" {
            // A background review poll owns the single response slot. Queue
            // this confirmed local action instead of dropping it while busy;
            // the pump gives it priority over the next idle poll.
            self.leaveRequested = true;
            ui.actionError("");
            return true;
        }
        let approve = match action {
            "edge_space_approve" => true,
            "edge_space_reject" => false,
            _ => return false,
        };
        if self.busy {
            return true;
        }
        let Some(request) = self.pending.clone() else {
            return true;
        };
        let service = self.service.clone();
        let sender = self.sender.clone();
        let scheduled = self.scheduler.scheduleHostRuntimeAsyncTask(
            "edge-space-decision",
            Box::new(move || {
                Box::pin(async move {
                    let result = service
                        .decideDeviceSpaceJoin(
                            request.requestId,
                            request.assignmentVersion,
                            approve,
                        )
                        .await;
                    let _ = sender.try_send(Event::Decision(result));
                })
            }),
        );
        match scheduled {
            Ok(()) => {
                self.busy = true;
                ui.setSpaceJoinPrompt("", true);
            }
            Err(error) => {
                log::error!("Space review schedule: {}", error.message);
            }
        }
        true
    }
    pub fn pump(&mut self, ui: &mut Esp32Ui) {
        while let Ok(event) = self.receiver.try_recv() {
            self.busy = false;
            match event {
                Event::Ready(Ok(())) => {
                    self.ready = true;
                    self.initialization = None;
                    log::info!("operit-esp32 shared Space admission ready");
                }
                Event::Ready(Err(error)) => {
                    log::error!("Space initialization: {error}");
                    ui.actionError("空间初始化失败，正在重试");
                }
                Event::Poll(Ok(request)) => {
                    if !self.leaveRequested && self.pending != request {
                        self.pending = request;
                        let text = self
                            .pending
                            .as_ref()
                            .map(|r| {
                                format!(
                                    "申请加入空间\n{}\n{}",
                                    r.applicantName.chars().take(32).collect::<String>(),
                                    r.spaceName.chars().take(32).collect::<String>()
                                )
                            })
                            .unwrap_or_default();
                        ui.setSpaceJoinPrompt(&text, false);
                    }
                }
                Event::Left(Ok(())) => {
                    self.pending = None;
                    ui.setSpaceJoinPrompt("", false);
                    ui.actionError("");
                    crate::edge_chat::clear();
                    log::info!("Space left; direct pairing retained");
                }
                Event::Left(Err(error)) => {
                    log::error!("Space exit: {error}");
                    ui.actionError(&error);
                }
                Event::Poll(Err(error)) => {
                    log::warn!("Space review poll: {error}");
                }
                Event::Decision(Ok(request)) => {
                    log::info!("Space review completed: {:?}", request.status);
                    self.pending = None;
                    ui.setSpaceJoinPrompt("", false);
                }
                Event::Decision(Err(error)) => {
                    log::error!("Space review failed (request retained): {error}");
                    // Keep the captured request/version and real approval controls.
                    let message = if error.contains("NVS capacity")
                        || error.contains("NOT_ENOUGH_SPACE")
                        || error.contains("NVS budget")
                    {
                        "设备存储空间不足
申请已保留，暂未完成审批"
                    } else {
                        "审批未完成，申请已保留
请重试"
                    };
                    ui.setSpaceJoinPrompt(message, false);
                }
            }
            self.nextPoll = Instant::now() + Duration::from_secs(3);
        }
        if !self.ready && !self.busy && Instant::now() >= self.nextPoll {
            if let Some((router, profile)) = self.initialization.clone() {
                let service = self.service.clone();
                let sender = self.sender.clone();
                let scheduled = self.scheduler.scheduleHostRuntimeTask(
                    "edge-space-init-retry",
                    Box::new(move || {
                        let initialized = service
                            .initialize(profile)
                            .and_then(|_| router.installSpace(service));
                        let _ = sender.try_send(Event::Ready(initialized));
                    }),
                );
                if scheduled.is_ok() {
                    self.busy = true;
                }
                self.nextPoll = Instant::now() + Duration::from_secs(3);
            }
        }
        if self.ready && !self.busy && self.leaveRequested {
            let service = self.service.clone();
            let sender = self.sender.clone();
            match self.scheduler.scheduleHostRuntimeTask(
                "edge-space-leave",
                Box::new(move || {
                    let result = service.leaveDeviceSpace().map(|_| ());
                    let _ = sender.try_send(Event::Left(result));
                }),
            ) {
                Ok(()) => {
                    self.busy = true;
                    self.leaveRequested = false;
                }
                Err(error) => {
                    self.leaveRequested = false;
                    ui.actionError(&error.message);
                }
            }
        }
        if self.ready && !self.busy && Instant::now() >= self.nextPoll {
            let service = self.service.clone();
            let sender = self.sender.clone();
            let scheduled = self.scheduler.scheduleHostRuntimeAsyncTask(
                "edge-space-incoming",
                Box::new(move || {
                    Box::pin(async move {
                        let result = service.incomingDeviceSpaceJoins().await.map(|requests| {
                            requests.into_iter().find(|request| request.canApprove)
                        });
                        let _ = sender.try_send(Event::Poll(result));
                    })
                }),
            );
            if scheduled.is_ok() {
                self.busy = true;
            }
            self.nextPoll = Instant::now() + Duration::from_secs(3);
        }
    }
}
