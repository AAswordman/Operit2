//! Exercises the shared firmware/simulator session lifecycle, not a second UI implementation.
use super::*;
use async_trait::async_trait;
use operit_link::*;
use operit_edge_sdk::runtime::{NodeServices::*, RuntimePeerService::RuntimePeerService};
use operit_peer_link::{PeerEndpoint, PeerListenerCapabilities, PeerTransport};
use std::collections::BTreeSet;

type Ack = tokio::sync::oneshot::Receiver<Result<CoreValue, CoreLinkError>>;
#[derive(Default)]
struct Client {
    calls: Mutex<Vec<CoreCallRequest>>,
    watches: Mutex<
        Vec<(
            CoreWatchRequest,
            tokio::sync::mpsc::UnboundedSender<CoreEvent>,
        )>,
    >,
    ack: Mutex<Option<Ack>>,
}
impl Client {
    fn gated() -> (
        Arc<Self>,
        tokio::sync::oneshot::Sender<Result<CoreValue, CoreLinkError>>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (
            Arc::new(Self {
                ack: Mutex::new(Some(rx)),
                ..Self::default()
            }),
            tx,
        )
    }
}
#[async_trait(?Send)]
impl CoreLinkSharedClient for Client {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let id = request.requestId.clone();
        self.calls.lock().unwrap().push(request);
        let ack = self.ack.lock().unwrap().take();
        CoreCallResponse {
            requestId: id,
            result: match ack {
                Some(ack) => ack.await.unwrap(),
                None => Ok(CoreValue::Null),
            },
        }
    }
    async fn watchSnapshot(&self, _: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        panic!("unexpected snapshot")
    }
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (tx, stream) = CoreEventStream::channel();
        // A real bounded window descriptor is needed to exercise embedded
        // stream reopening with source arguments, not just primary watches.
        let value = if request.propertyName == "chatMessagesWindowFlow" {
            operit_link::toCoreValue(serde_json::json!({"messages":[{
                "sender":"ai", "text":"", "contentStream":{"$coreStream":{
                    "streamId":"reply", "target":CORE_STREAM_TARGET, "propertyName":"events", "args":{}
                }}
            }], "older":null, "error":null})).unwrap()
        } else if request.target == CORE_STREAM_TARGET {
            operit_link::toCoreValue(ChatDisplayEvent::Chunk { value: String::new() }).unwrap()

        } else {
            CoreValue::emptyMap()
        };
        tx.send(CoreEvent {
            requestId: None,
            target: request.target.clone(),
            propertyName: request.propertyName.clone(),
            kind: CoreEventKind::Snapshot,
            value,
        })
        .unwrap();
        self.watches.lock().unwrap().push((request, tx));
        Ok(stream)
    }

    /// Fails when the chat fixture receives an unexpected input-stream request.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        panic!("unexpected input stream: {}", request.methodName)
    }
}
#[derive(Default)]
struct Peers {
    entry: Mutex<Option<SpaceClientConnection>>,
}
impl Peers {
    fn set(&self, space: &str, peer: &str, generation: &str, client: Arc<Client>) {
        *self.entry.lock().unwrap() = Some(SpaceClientConnection {
            identity: SpaceClientIdentity {
                spaceId: space.into(),
                peerNodeId: peer.into(),
                generation: generation.into(),
            },
            client,
        });
    }
}
#[async_trait(?Send)]
impl RuntimePeerService for Peers {
    fn spaceConnection(&self) -> Option<SpaceClientConnection> {
        self.entry
            .lock()
            .unwrap()
            .as_ref()
            .map(|e| SpaceClientConnection {
                identity: e.identity.clone(),
                client: e.client.clone(),
            })
    }
    async fn discoverPeers(&self, _: u64) -> Result<Vec<DiscoveredPeer>, CoreLinkError> {
        panic!("unused")
    }
    /// Rejects pairing requests outside this chat-session fixture contract.
    async fn startPairing(
        &self,
        _: PeerEndpoint,
        _: Option<PeerTransport>,
    ) -> Result<PendingPairing, CoreLinkError> {
        panic!("unused")
    }
    async fn finishPairing(&self, _: &str, _: &str) -> Result<PairedPeer, CoreLinkError> {
        panic!("unused")
    }
    async fn cancelPairing(&self, _: &str) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
    fn listenerCapabilities(&self) -> PeerListenerCapabilities {
        PeerListenerCapabilities {
            transports: vec![],
            discoveryAdvertisement: false,
        }
    }
    async fn startListening(&self, _: &[PeerTransport]) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
    async fn stop(&self) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
    async fn call(&self, _: &str, _: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
        panic!("unused")
    }
    async fn watchSnapshot(
        &self,
        _: &str,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        panic!("unused")
    }
    async fn watch(
        &self,
        _: &str,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        panic!("unused")
    }
    async fn openPush(
        &self,
        _: &str,
        _: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        panic!("unused")
    }
    fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError> {
        Ok(Vec::new())
    }
    fn outboundPeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> {
        Ok(BTreeSet::new())
    }
    fn activePeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> {
        Ok(["paired-but-not-admitted".into()].into())
    }
    fn subscribePeerChanges(&self) -> tokio::sync::broadcast::Receiver<()> {
        tokio::sync::broadcast::channel(1).1
    }
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> {
        Ok(Vec::new())
    }
    async fn disconnectPeer(&self, _: &str) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
    async fn removePairedPeer(&self, _: &str) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
    fn forgetPairingRecords(&self, _: &[String]) -> Result<(), CoreLinkError> {
        panic!("unused")
    }
}
async fn until(condition: impl Fn() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !condition() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("session transition timed out");
}
fn session() -> Arc<ChatSession> {
    SESSION.get().unwrap().lock().unwrap().clone().unwrap()
}
fn assertWatches(client: &Client, chat: &str) {
    let watches = client.watches.lock().unwrap();
    for method in [
        "chatMessagesWindowFlow",
        "chatStateFlow",
        "events",
    ] {
        let request = &watches
            .iter()
            .find(|(r, _)| r.propertyName == method)
            .expect("missing restored subscription")
            .0;
        let CoreValue::Map(args) = &request.args else {
            panic!()
        };
        if method == "events" {
            assert_eq!(request.target, CORE_STREAM_TARGET);
            assert_eq!(
                args[CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT],
                CoreValue::String("watch".into())
            );
            assert_eq!(
                args[CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT],
                CoreValue::String("chatMessagesWindowFlow".into())
            );
            let CoreValue::Map(source) = &args[CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT] else {
                panic!()
            };
            assert_eq!(source["chatId"], CoreValue::String(chat.into()));
        } else {
            assert_eq!(args["chatId"], CoreValue::String(chat.into()));
        }
    }
}
#[tokio::test]
async fn provision_reconnect_and_restore_without_local_business_state_or_mutation_replay() {
    let scheduler =
        Arc::new(operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost::new().unwrap());
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler);
    clear();
    let peers = Arc::new(Peers::default());
    let services = NodeServices::new(peers.clone());
    let mut route = SpaceChatRoute::default();
    route.poll(services.clone());
    assert!(
        !isConnected(),
        "paired traffic alone is not a usable Space entry"
    );
    let (first, lateAck) = Client::gated();
    peers.set("space", "core", "1", first.clone());
    route.poll(services.clone());
    until(|| first.calls.lock().unwrap().len() == 1).await;
    let id = session().chatId.clone();
    assert!(uuid::Uuid::parse_str(&id).is_ok());
    assert_eq!(
        first.calls.lock().unwrap()[0].methodName,
        "ensureRoutedChat"
    );
    assert!(
        first.watches.lock().unwrap().is_empty(),
        "no watches before object creation is acknowledged"
    );
    assert!(send("before ack".into()).is_err());
    // Replace the entry without observing None. The old acknowledgement cannot
    // revive obsolete subscriptions, and retries retain the same creation key.
    let (second, ack) = Client::gated();
    peers.set("space", "core", "2", second.clone());
    route.poll(services.clone());
    until(|| second.calls.lock().unwrap().len() == 1).await;
    assert_eq!(session().chatId, id);
    lateAck.send(Ok(CoreValue::Null)).unwrap();
    ack.send(Ok(CoreValue::Null)).unwrap();
    until(|| second.watches.lock().unwrap().len() == 3).await;
    assert!(first.watches.lock().unwrap().is_empty());
    assert!(second.watches.lock().unwrap().iter().all(|(r,_)| r.propertyName != "routedChatListFlow"), "metadata must stay unloaded until browsing");
    assertWatches(&second, &id);
    let stable = session();
    route.poll(services.clone());
    assert!(
        Arc::ptr_eq(&stable, &session()),
        "normal polling must not reopen subscriptions"
    );
    // Move to another Core in the same Space, preserving the selected chat.
    let third = Arc::new(Client::default());
    peers.set("space", "other-core", "3", third.clone());
    route.poll(services.clone());
    until(|| third.watches.lock().unwrap().len() == 3).await;
    assert_eq!(session().chatId, id);
    assertWatches(&third, &id);
    until(|| {
        second
            .watches
            .lock()
            .unwrap()
            .iter()
            .all(|(_, tx)| tx.is_closed())
    })
    .await;
    // A lost logical watch must be restored even if the entry stays unchanged.
    let (r, tx) = third
        .watches
        .lock()
        .unwrap()
        .iter()
        .find(|(r, _)| r.propertyName == "chatStateFlow")
        .unwrap()
        .clone();
    tx.send(CoreEvent {
        requestId: None,
        target: r.target,
        propertyName: r.propertyName,
        kind: CoreEventKind::Completed,
        value: CoreValue::Null,
    })
    .unwrap();
    until(|| session().needsReconnect.load(Ordering::Acquire)).await;
    route.lastAttempt = Some(std::time::Instant::now() - std::time::Duration::from_secs(4));
    route.poll(services.clone());
    until(|| third.watches.lock().unwrap().len() == 6).await;
    assert_eq!(
        third.calls.lock().unwrap().len(),
        1,
        "watch recovery does not create new chats or replay sends"
    );
    // Offline/reconnect in the same Space only restores volatile display flows.
    *peers.entry.lock().unwrap() = None;
    route.poll(services.clone());
    assert!(!isConnected());
    peers.set("space", "other-core", "3", third.clone());
    route.poll(services.clone());
    until(|| third.watches.lock().unwrap().len() == 9).await;
    assert_eq!(session().chatId, id);
    assert_eq!(third.calls.lock().unwrap().len(), 1);
    // A different Space must not inherit the previous Space's chat ID.
    let (fourth, rejected) = Client::gated();
    peers.set("new-space", "core", "4", fourth.clone());
    route.poll(services.clone());
    until(|| fourth.calls.lock().unwrap().len() == 1).await;
    let nextId = session().chatId.clone();
    assert_ne!(id, nextId);
    rejected
        .send(Err(CoreLinkError::new("ROUTE_PERMISSION_DENIED", "denied")))
        .unwrap();
    until(|| session().needsReconnect.load(Ordering::Acquire)).await;
    assert!(fourth.watches.lock().unwrap().is_empty());
    assert!(!session().ready.load(Ordering::Acquire));
    // Idempotent initialization can be retried with the same ID, never by
    // repurposing an ordinary read/send as a Binding-creation operation.
    route.lastAttempt = Some(std::time::Instant::now() - std::time::Duration::from_secs(4));
    route.poll(services);
    until(|| fourth.watches.lock().unwrap().len() == 3).await;
    assert_eq!(session().chatId, nextId);
    route.reset();
    until(|| {
        fourth
            .watches
            .lock()
            .unwrap()
            .iter()
            .all(|(_, tx)| tx.is_closed())
    })
    .await;
    assert!(!isConnected());
}
