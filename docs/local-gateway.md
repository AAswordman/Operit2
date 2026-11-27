# 本地网关供应商

在运行 Magpie、CC Switch 等 HTTP 网关的设备上，在模型供应商中选择 **本地网关**（`LOCAL_GATEWAY`），填写网关地址、密钥和模型。供应商创建时会将其 ID 作为 Binding key，绑定当前 CoreNode。供应商配置与 Binding 按现有设备空间机制同步。

例如，在电脑上填写 `http://127.0.0.1:8000/v1/responses`。手机同步并选用该供应商后，请求通过 Core route 发给电脑，由电脑的 HttpHost 访问该地址。无需把网关监听地址改成公网地址。电脑需运行 Operit Core、保持设备空间路由连通，并授予调用设备 `target:runtime.execute` 所要求的执行权限。

接口路径明确指定协议：

| 地址结尾 | 协议 |
| --- | --- |
| 主机地址、`/v1`、`/chat/completions` | OpenAI Chat Completions |
| `/responses` | OpenAI Responses |
| `/messages` | Anthropic Messages |

模型列表通过 `listLocalGatewayModels` 路由在网关设备读取。消息、连接测试、密钥测试通过 `openLocalGatewayResponse` 路由传输 HTTP 状态、响应头、字节块和结束结果；现有协议供应商负责序列化、工具调用、思考内容、用量和流式解析。取消请求会释放 route Watch，并关闭网关设备的 Host HTTP 流。

路由仅承载模型请求；会话存储和工具执行保留其既有路由。目标设备校验 Binding 归属、供应商类型及请求地址与供应商配置一致。离线、权限不足、网关 HTTP 错误和流中断均作为请求错误上报。

更改已有本地网关的地址不会改变设备归属。需要换到另一台设备时，在那台设备重新创建本地网关供应商。

新增路由将 Core / Edge 协议版本提升为 v30。参与连接的设备需更新到包含此协议版本的版本后再配对、重连和使用网关路由。
