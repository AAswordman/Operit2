# 外部连接器

外部连接器是独立的设备级插件，不需要安装「维护中控」。安装并启用 `com.operit.connector_hub`，再按需启用 QQ Bot、微信 Bot 或飞书 Bot。不同渠道缺失、停用或报错不会阻塞其他渠道。

1. 从主侧边栏的「插件」区或工具箱打开「外部连接器」。总览显示各 Bot 的连接状态，顶部可直接切换到 QQ、微信或飞书。微信扫码后开始监听；QQ 和飞书输入应用的 App ID、App Secret，校验成功后开始监听。QQ、微信页的「高级设置」直达对应插件原有的控制界面。
2. 给 Bot 发第一条消息。控制台会发现会话，可按渠道或名称查找，无需填写用户、群或会话 ID。
3. 打开会话，查看近期收发记录。可回复最近一条入站消息，也可指定一条消息，附上设备文件。自动回复单独开启；不会因新接入自动启用。原来已开启的设置会保留。
4. 若还使用维护中控，在原来的 Bot 软件聊天绑定工作区并启用维护即可。不需要再创建一套渠道配置。只用软件内反馈时，也可以不启用外部连接器。

控制台不是平台历史归档：只显示本机近期入站消息、附件摘要，以及该版本开始由 Bot 向平台提交的消息（包括自动回复、维护通知和手动发送）。出站记录区分「平台已接受」「平台拒绝」和「发送结果未知」，不代表对方已读。单次回复拆成多条时分别记录；超时、断线等不确定结果不会自动重发。旧版控制台发送记录仍可查看。微信原生语音需要指定时长、编码和采样率；控制台把普通音频作为文件发送。飞书 MP4 可直接按原生视频发送，封面可选。其他平台媒体权限、窗口和格式限制仍适用。

近期收到的附件可从会话中手动保存，单件上限 8 MiB。保存入口只接受当前账户、当前会话中的真实消息；过期附件或权限不足时不会返回原始地址或密钥。旧记录可能没有可保存的附件。

## 插件接口

在 manifest 声明 `com.operit.connector_hub`（最低 `1.0.0`），使用自包含的 `src/api.ts` 调用 `connectors.v1`。例如：

```ts
const { connectors, conversations } = await request({ action: "dashboard" });
const session = conversations[0];
if (session) {
  const history = await request({ action: "history", channel: session.channel, conversationKey: session.key });
  if (history.latestReplyKey) await request({ action: "reply", channel: session.channel, key: history.latestReplyKey, text: "已收到" });
}
```

`open_chat` 复用 Bot 管理的软件聊天；`subscribe`、`peek`、`ack` 使用宿主提供的调用包身份作为订阅命名空间，调用方不能指定其他消费方。`reply`、`send` 每次提交一种内容；QQ 和飞书可用 `send` 向已发现会话主动发送。微信需要用 `reply` 回复已收到的消息。控制台的 QQ 会话默认主动发送，点选「引用回复」后引用指定消息。结构化能力见各渠道插件。管理和凭据操作仅供控制台内部使用，公开接口不返回上下文令牌、原始媒体密钥或平台原始响应。
