# A2A 扩展与包管理

A2A 是扩展目录中的第五种类型，与插件、脚本包、技能、MCP 并列，界面统一显示 `A2A`。
它接入 OPR 的发现、发布、安装和更新流程。外部 Agent 不成为设备空间的 CoreNode；设备空间
只决定连接配置在哪些 Operit 设备之间共享。

## 安装所有权

每个连接由 `ExtensionStore` 的 `kind = "a2a"` 记录持有，使用既有的安装、删除、作用域迁移和同步
机制，不建立第二份前端安装状态：

```text
runtime/extensions/{device|space}/records/a2a-{sha256(id)}.json
  settings.agent = { url, headers, disabled, card }
  settings.marketEntryId = 可选的 OPR 条目 ID
  settings.installedMarketVersionId = 可选的已安装版本 ID
```

手动导入默认属于本机，详情页可以迁移到设备空间。共享时连接配置（包括请求头）遵循所选安装
作用域，与远程 MCP 的配置模型一致。市场版本只提供公共接入配置；用户在自己的安装里配置认证。
市场更新保留原有作用域、启用状态以及同源服务的用户请求头，并拒绝覆盖其他条目或手动安装拥有的 ID。

## 包与工具

`A2ARepository` 负责连接清单、Agent Card 发现和协议调用。`RuntimePackageManager` 把每个安装
投影为 `category = "A2A"`、名称为 `a2a.{id}` 的 `ToolPackage`，能力描述来自 Agent Card。
包扫描重建时也重建该投影，因此重启后仍可发现。

A2A 使用现有的启用包列表、`use_package`、工具搜索、`package_proxy` 和工具执行管线：

| 包工具 | 作用 |
| --- | --- |
| `a2a.{id}:send_message` | 创建任务或用 `task_id`、`context_id` 补充消息 |
| `a2a.{id}:get_task` | 查询状态、历史消息和交付物 |
| `a2a.{id}:cancel_task` | 请求远端取消任务，可能被拒绝 |

`A2AToolExecutor` 在每次调用时读取当前安装，停用和删除立即阻止后续请求。通用包管理 API 的启用、
停用、删除、作用域迁移也支持这些包。A2A 不作为 JavaScript 脚本再次导出或在扩展列表重复出现。
设备空间中提交的 A2A 记录变更只刷新 A2A 包投影，并触发既有扩展目录修订通知。

## 协议边界

首版实现 A2A **1.0 / 0.3 JSON-RPC**：按 Agent Card 声明选择支持的接口，使用各自的消息字段、
方法名、角色值和任务配置；1.0 支持 AgentInterface 的 tenant。服务根地址通过
`/.well-known/agent-card.json` 发现，也可显式填写 Agent Card JSON 地址。

发送请求允许立即返回未完成任务。返回的 JSON 保留远端任务 ID、上下文 ID、状态、消息和 artifacts，
AI 或测试任务界面可继续查询、回复补充输入或请求取消。任务结果沿用现有聊天工具结果的持久化，
不在共享连接清单内保存聊天内容。

HTTP 使用 Host 提供的异步响应流，支持各平台统一调用、取消清理、超时和响应大小限制。
发送和取消失败后不自动重试，以免重复提交任务。自动发现的跨源接口不会接收原地址的认证请求头。
本轮未实现 A2A 服务端、SSE 订阅、推送通知、gRPC 或 HTTP+JSON 绑定。

协议参考：
- https://a2a-protocol.org/latest/specification/
- https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto

## 导入与 OPR 清单

添加对话框支持服务地址、JSON 文本和 `.json` / `.a2a` 文件。OPR 使用同一个清单，条目类型为 `a2a`，
将清单放入仓库版本的 `installConfig`。Flutter 和 CLI 的市场安装均调用同一个 repository，安装版本
标记跟随所有权记录，因此迁移和删除后不会留下独立的市场安装标记。

```json
{
  "a2aAgents": {
    "research": {
      "url": "https://agent.example.com",
      "headers": {},
      "disabled": false
    }
  }
}
```

`card` 可省略，在读取或首次调用时获取；也可在清单里携带已经验证的 Agent Card，以便离线展示能力。
ID 为 1–100 个 ASCII 字母、数字、点、下划线或短横线。UI 通过生成的 CoreProxy 调用 Core，
不直接联网或自行维护安装状态。市场后端需要接受 `a2a` 类型及对应分类才能在线发布该类条目。
