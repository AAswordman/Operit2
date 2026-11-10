//! Exercise production hello constructors through the same binary Link codec used by carriers.
use super::*;

struct WirePeer {
    source: PeerEndpoint,
    target: PeerEndpoint,
    tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    rx: AsyncMutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
    omitIdentity: bool,
    ownedDecode: bool,
}
#[async_trait]
impl PeerConnection for WirePeer {
    fn source(&self) -> &PeerEndpoint { &self.source }
    fn target(&self) -> &PeerEndpoint { &self.target }
    fn transport(&self) -> PeerTransport { PeerTransport::Tcp }
    async fn send(&self, mut message: PeerMessage) -> Result<(), String> {
        if self.omitIdentity {
            match &mut message {
                PeerMessage::Request(CoreLinkRequest::Call(request)) if request.methodName == "hello" => {
                    let CoreValue::Map(fields) = &mut request.args else { panic!("hello must be a map") };
                    assert!(fields.remove("protocol").is_some());
                }
                PeerMessage::Response(CoreLinkResponse::Call(response)) => {
                    if let Ok(value) = &mut response.result {
                        if let CoreValue::Map(fields) = value {
                            if let Some(CoreValue::Map(transcript)) = fields.get_mut("transcript") {
                                assert!(transcript.remove("protocol").is_some());
                            }
                        }
                    }
                }
                _ => {},
            }
        }
        self.tx.send(encodeLink(message).map_err(|error| error.to_string())?).map_err(|error| error.to_string())
    }
    async fn receive(&self) -> Result<Option<PeerMessage>, String> {
        self.rx.lock().await.recv().await.map(|bytes| {
            if self.ownedDecode { PeerMessage::decodeOwned(bytes.into()) } else { PeerMessage::decode(&bytes) }
                .map_err(|error| error.to_string())
        }).transpose()
    }
    async fn close(&self) { self.rx.lock().await.close(); }
}
fn service(node: &str, platform: &str) -> HostRuntimePeerService {
    let mut service = super::single_channel_tests::service();
    let state = Arc::get_mut(&mut service.state).unwrap();
    state.nodeId = node.into();
    state.info.platform = platform.into();
    service
}
fn wires(omitRequest: bool, omitReply: bool) -> (Arc<dyn PeerConnection>, Arc<dyn PeerConnection>) {
    let (a, rxA) = tokio::sync::mpsc::unbounded_channel();
    let (b, rxB) = tokio::sync::mpsc::unbounded_channel();
    let endpoint = |node: &str| PeerEndpoint { nodeId: node.into(), address: "wire-test".into() };
    (Arc::new(WirePeer { source: endpoint("initiator"), target: endpoint("receiver"), tx: a,
        rx: AsyncMutex::new(rxB), omitIdentity: omitRequest, ownedDecode: false }),
     Arc::new(WirePeer { source: endpoint("receiver"), target: endpoint("initiator"), tx: b,
        rx: AsyncMutex::new(rxA), omitIdentity: omitReply, ownedDecode: true }))
}

#[tokio::test]
async fn current_hello_and_reply_preserve_identity_over_binary_link_in_both_directions() {
    for (from, to) in [("ios", "macos"), ("macos", "ios")] {
        let initiator = service("initiator", from);
        let receiver = service("receiver", to);
        let (outbound, inbound) = wires(false, false);
        let (started, served) = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::join!(initiator.handshake(outbound, "start", "", None, None, None), receiver.serve(inbound))
        }).await.expect("current hello/reply must complete without a protocol rejection");
        served.unwrap();
        let (_, id, info, _, node) = started.unwrap();
        assert_eq!(node, "receiver");
        assert_eq!(info.platform, to);
        let pending = receiver.pending(true, &id).unwrap();
        assert!(pending.confirmationCode.is_some(), "receiver must create the code after the wire handshake");
    }
}

#[tokio::test]
async fn omitted_request_or_reply_identity_reports_which_side_rejected_the_handshake() {
    for (request, reply, phase) in [(true, false, "received hello"), (false, true, "received hello reply")] {
        let initiator = service("initiator", "ios");
        let receiver = service("receiver", "macos");
        let (outbound, inbound) = wires(request, reply);
        let failure = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let start = initiator.handshake(outbound, "start", "", None, None, None);
            let serve = receiver.serve(inbound);
            tokio::pin!(start, serve);
            tokio::select! {
                result = &mut start => result,
                result = &mut serve => { result.unwrap(); start.await }
            }
        }).await.unwrap().err().expect("missing identity must fail closed");
        assert_eq!(failure.code, "PEER_PROTOCOL_UPGRADE_REQUIRED");
        let details: serde_json::Value = fromCoreValue(failure.details.unwrap()).unwrap();
        assert_eq!(details["phase"], phase);
        assert_eq!(details["localProtocol"], crate::PeerProtocol::VERSION);
        assert!(receiver.state.store.records::<Pending>(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH).unwrap().is_empty());
    }
}
