//! Runtime 节点通信入口：供 Router 转发调用，供应用管理监听和配对。
//! PeerLink 只负责连接和传输；匿名准入、身份校验、配对状态和持久化由 runtime 负责。
//! 配对使用标准 Link Call，不定义额外消息协议，不设置 HTTP 专用配对接口。

use super::NodeServices::{PairedPeer, PairingDirection, PairingPrompt, PendingPairing};
use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CorePushRequest, CoreWatchRequest, RoutedCoreRequest,
};
use operit_peer_link::{PeerEndpoint, PeerTransport};

/// 一个本地节点的通信服务；Router 不接触 socket、连接收发或配对握手。
/// 生产实现见 HostRuntimePeerService；契约不依赖具体传输。
#[async_trait(?Send)]
pub trait RuntimePeerService: Send + Sync {
    /// Volatile, admitted Space return route; not reverse pairing permission.
    fn spaceClient(&self) -> Option<std::sync::Arc<dyn operit_link::CoreLinkSharedClient + Send + Sync>> { None }

    /// Selects an admitted entry together with its volatile identity for UI reconnection.
    fn spaceConnection(&self) -> Option<super::NodeServices::SpaceClientConnection> { None }

    /// Returns unpaired local-discovery candidates for every application using this Core.
    /// Excludes local and paired node IDs in either authorization direction, including offline peers.
    /// Discovery is not authentication or Space membership proof; candidates still require pairing.
    async fn discoverPeers(
        &self,
        timeoutMs: u64,
    ) -> Result<Vec<super::NodeServices::DiscoveredPeer>, CoreLinkError>;

    /// Starts pairing over PeerLink using ephemeral keys and identity-bound proofs.
    /// Every transport requires confirmation with the code shown by the receiving device.
    /// Pairing grants no Space membership or business-call authorization.
    /// transport 为 None 时按地址自动判定载体：URL scheme 直接映射，
    /// 裸 `ip:port` 用一次性探测连接分类。
    async fn startPairing(
        &self,
        target: PeerEndpoint,
        transport: Option<PeerTransport>,
    ) -> Result<PendingPairing, CoreLinkError>;

    /// 按配对标识取出本地待确认状态，通过同一 Link Call 流程提交确认码。
    /// 授权方向分别记录；一次配对不自动创建反向授权，按设备撤销时则同时清除两边。
    /// 单向解除后允许重新向仍有旧记录的接收端配对；仅新配对码验证成功后，
    /// 接收端替换该设备的旧凭证并关闭旧会话，不改变 Space 成员或审批规则。
    /// 必要时使用已保存的端点和传输方式重新连接；双方身份与证明校验通过后才保存配对。
    /// 配对成功不等于获得 Space 成员资格，也不等于获得任意业务调用权限。
    async fn finishPairing(
        &self,
        pairingId: &str,
        confirmationCode: &str,
    ) -> Result<PairedPeer, CoreLinkError>;

    /// 取消本地待确认事务并清除临时秘密、释放其连接；不删除已有配对。
    /// 这是本地管理操作，不能仅凭远端传入的配对标识匿名执行。
    async fn cancelPairing(&self, pairingId: &str) -> Result<(), CoreLinkError>;

    /// Reports independent listener and discovery capabilities without opening resources.
    fn listenerCapabilities(&self) -> operit_peer_link::PeerListenerCapabilities;

    /// 启动指定传输的接收服务；内部调用 PeerLink::listen 并管理 accept/receive。
    /// bindAddress 和发现选项从 runtime 持有的原配置文件读取，不另建一套默认配置。
    /// 未鉴权连接只能进入 runtime 的匿名配对 Call 白名单；不向应用暴露监听器。
    /// 已鉴权业务交给 Router 检查路由与权限；配对和业务共享接收入口。
    async fn startListening(&self, transports: &[PeerTransport]) -> Result<(), CoreLinkError>;

    /// 停止监听并关闭由本服务管理的连接和流；不删除持久化配对记录。
    async fn stop(&self) -> Result<(), CoreLinkError>;

    /// Submits a routed call to the adjacent node selected by the Router.
    /// nextNodeId 是相邻节点，request.targetNodeId 是最终节点，二者不能混淆。
    /// 内部根据配对记录选择端点和传输，建立连接、完成鉴权、发送 Call 并关联响应。
    /// 原调用方、Space、路由种类和 TTL 必须保留；不在此重新选路或执行 Proxy。
    async fn call(
        &self,
        nextNodeId: &str,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse;

    /// 与 call 使用同一已鉴权连接入口，返回标准 Watch 快照。
    async fn watchSnapshot(
        &self,
        nextNodeId: &str,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// 与 call 使用同一入口，返回标准事件流；关联、取消和断线清理由内部处理。
    async fn watch(
        &self,
        nextNodeId: &str,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError>;

    /// 与 call 使用同一入口，返回标准 Push 会话；Router 不处理底层帧和连接。
    async fn openPush(
        &self,
        nextNodeId: &str,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;

    /// 列出本节点已配对的对端；同一对端的传输渠道不另算一个配对身份。
    fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError>;

    /// Uses the current pairing projection without inferring reverse authorization.
    fn hasPairing(&self, node: &str, direction: PairingDirection) -> Result<bool, CoreLinkError> {
        Ok(self.pairedPeers()?.iter().any(|peer| {
            peer.nodeId == node && peer.authorizes(direction)
        }))
    }

    /// 返回允许本节点主动调用的已配对身份；按节点去重，不把入站授权推导成出站授权。
    /// 多个传输渠道属于同一个身份；调用方看不到端点、会话或密钥。
    fn outboundPeerNodeIds(&self) -> Result<std::collections::BTreeSet<String>, CoreLinkError>;

    /// 当前有已认证在线证据的相邻节点，不包含匿名连接。
    /// 同空间回连通过独立的空间凭证建立，不修改普通配对方向。
    fn activePeerNodeIds(&self) -> Result<std::collections::BTreeSet<String>, CoreLinkError>;

    /// 订阅待确认配对、授权或可用性变化；先订阅再读取快照，避免启动时漏掉变化。
    /// 通知不含传输细节；落后接收者重新读取快照，停止服务时唤醒接收者。
    fn subscribePeerChanges(&self) -> tokio::sync::broadcast::Receiver<()>;

    /// 本地 UI 读取待确认请求和验证码；不得向未鉴权远端暴露。
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError>;

    /// 关闭该节点所有渠道的当前连接，保留配对授权；由统一连接所有者执行。
    async fn disconnectPeer(&self, peerNodeId: &str) -> Result<(), CoreLinkError>;

    /// 按设备撤销全部入站/出站授权、所有传输渠道及关联凭证；不让用户选择方向。
    /// 这是需要本地管理权限的单向本地操作，不通知对端，不属于匿名配对入口。
    async fn removePairedPeer(&self, peerNodeId: &str) -> Result<(), CoreLinkError>;

    /// 同步撤销这些设备的配对凭证（入站/出站授权与关联会话），不等待关闭在用渠道。
    /// 遗忘必须发生在成员状态变化的那一刻：延迟执行无法区分旧配对与重新配对产生的
    /// 新配对（记录里没有实例标识），会把重新入网时刚建立的边一起删掉。
    fn forgetPairingRecords(&self, nodeIds: &[String]) -> Result<(), CoreLinkError>;
}
