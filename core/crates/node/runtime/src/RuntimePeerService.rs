//! Runtime 节点通信入口：供 Router 转发调用，供应用管理监听和配对。
//! PeerLink 只负责连接和传输；匿名准入、身份校验、配对状态和持久化由 runtime 负责。
//! 配对使用标准 Link Call，不定义额外消息协议，不设置 HTTP 专用配对接口。

use super::NodeServices::{PairedPeer, PairingPrompt, PendingPairing};
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

    /// Captures one admitted Space generation for caller-owned binary Push streams.
    fn spacePushClient(&self) -> Option<std::sync::Arc<dyn operit_link::CoreLinkSpacePushClient>> { None }

    /// Selects an admitted entry together with its volatile identity for UI reconnection.
    fn spaceConnection(&self) -> Option<super::NodeServices::SpaceClientConnection> { None }

    /// Returns unpaired local-discovery candidates for every application using this Core.
    /// Excludes local and paired node IDs in either authorization direction, including offline peers.
    /// Discovery is not authentication or Space membership proof; candidates still require pairing.
    async fn discoverPeers(
        &self,
        timeoutMs: u64,
    ) -> Result<Vec<super::NodeServices::DiscoveredPeer>, CoreLinkError>;

    /// 使用 PeerLink::connect(host, source, target, transport) 连接目标并发起配对。
    /// LAN 发现配对允许 token 为 None：由 runtime 选择匿名配对入口，或从 Host 的受信本地凭证接口取得
    /// token；这不等于信任远端，仍必须经过配对码、身份绑定和密钥证明。
    /// 非局域网配对必须先验证 token，再进入配对码确认；缺失或错误必须拒绝。
    /// None 仅用于经 Host 实际来源和策略确认的 LAN 流程，不授权非 LAN 配对或业务访问。
    /// 是否允许免 token 配对由 Host 的实际接入来源和本地监听策略判断，不能仅信任目标地址字符串。
    /// 使用新鲜临时密钥、随机 nonce 和身份绑定证明；验证密钥交换及密钥确认后建立加密会话。
    /// 非对称密钥交换不等于消息已加密；会话必须使用经过验证的加密实现，不能以签名代替加密。
    /// 通过标准 Call 请求配对；runtime 校验响应及请求关联，保存待确认状态。
    /// target 中的节点标识只是寻址信息，不是身份已获验证的证据。
    /// 连接的取消与释放由 runtime 管理；不为 HTTP、WS、TCP、串口分别编写配对流程。
    async fn startPairing(
        &self,
        target: PeerEndpoint,
        transport: PeerTransport,
        token: Option<&str>,
    ) -> Result<PendingPairing, CoreLinkError>;

    /// 按配对标识取出本地待确认状态，通过同一 Link Call 流程提交确认码。
    /// token（如果有）已通过也不能跳过配对码；禁止旧 bootstrap/autoBootstrap 捷径。
    /// 授权方向分别记录；一次配对不自动创建反向授权，按设备撤销时则同时清除两边。
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
    /// bindAddress、token 和发现选项从 runtime 持有的原配置文件读取，不另建一套默认配置。
    /// 未鉴权连接只能进入 runtime 的匿名配对 Call 白名单；不向应用暴露监听器。
    /// 已鉴权业务交给 Router 检查路由与权限；配对和业务共享接收入口。
    async fn startListening(&self, transports: &[PeerTransport]) -> Result<(), CoreLinkError>;

    /// 停止监听并关闭由本服务管理的连接和流；不删除持久化配对记录。
    async fn stop(&self) -> Result<(), CoreLinkError>;

    /// Router 选出下一跳后直接调用此方法；不再查找旧 PeerLinkClient。
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
    /// 这是需要本地管理权限的操作，不属于匿名配对入口。
    async fn removePairedPeer(&self, peerNodeId: &str) -> Result<(), CoreLinkError>;
}
