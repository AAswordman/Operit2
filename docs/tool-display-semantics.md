# 工具展示语义

工具的展示信息由工具所有者声明，Core 在消息展示投影中传递，Flutter 负责渲染。工具名称、参数名称、运行平台和结果的自然语言文案都不用于推断展示语义。

## 内置工具

在 SDK 的 `ToolResultMap` 中，展示声明与工具名和结果类型放在一起：

```rust
#[display(action = "list", icon = "folder", target = "path", result = "directory")]
pub list_files: DirectoryListingData,
```

`tool_display` 宏生成 `BuiltinToolName::display()` 的精确查找。`action` 引用 SDK 的 `tool_display_labels.json` 本地化资源；新增动作时应同时增加对应资源。UI 不维护内置工具名称到展示样式的映射。

## 插件工具

在插件元数据的工具声明中添加可选的 `display`；条件状态中的工具也使用同一声明格式：

```json
{
  "name": "example",
  "tools": [
    {
      "name": "z42",
      "parameters": [
        {"name": "document", "type": "string", "required": true}
      ],
      "display": {
        "label": {"en": "List", "zh": "列出", "fr": "Lister", "default": "List"},
        "icon": "folder",
        "targetParameter": "document",
        "resultRenderer": "directory"
      }
    }
  ]
}
```

`label` 是语言标签到文本的映射。客户端依次选择完整语言标签、语言代码、`default`、`en`、首个声明文本；没有声明文本时显示原始工具名。`targetParameter` 是要显示在标题旁的参数键；完整参数仍可展开查看。

`resultRenderer` 明确选择结果视图：`text`（默认通用文本）、`directory`（目录）、`fileDiff`（文件差异）。缺省或未知值回退为通用文本。插件自行声明需要哪种视图；Flutter 不根据工具名、返回值的 `__type` 或对象是否含有某些字段自动选择视图。

目录视图读取 `{ "path": "/remote", "entries": [{ "name": "a.txt", "isDirectory": false, "size": 12 }] }`；文件差异视图读取 `{ "operation": { "path": "/remote/a.txt", "details": "Updated" }, "diffContent": "@@ -1 +1 @@\n-old\n+new" }`。插件返回这些普通 JSON 对象即可，不需要伪装成内置工具结果类型。声明只指定展示意图，数据仍须通过相应视图的结构校验；非法数据保留通用原文展示。

`icon` 是与平台无关的展示标识，目前支持 `file`、`folder`、`edit`、`delete`、`search`、`terminal`、`web`、`download`、`package`、`time`、`notification`、`clipboard`、`audio`。缺省或未知值使用通用工具图标。它不改变执行行为或权限。

没有 `display` 的工具使用通用展示。非法声明会使元数据加载返回错误，不会静默丢弃对应工具。Core 从当前运行时的有效插件声明读取信息，不向其他身份、节点或 Host 查询展示信息。

## 结果与消息链路

Core 保留原生 `ToolResultData`。Flutter 只按工具声明的 `resultRenderer` 选择结果视图，然后校验数据结构；不根据原生结果类型或类似 `Directory listing for ...` 的文本推断展示。目录来自执行 Host 返回的数据，不扫描 Flutter 所在设备。声明为 `fileDiff` 的工具还可以使用显式 `<file-diff>` 协议。

实时工具调用的展示声明附加在 Markdown XML 事件的 `display` 属性中。`proxy` / `package_proxy` 只按其明确的 `tool_name`、`params` 协议解析目标；目标尚未完整到达时使用通用展示。

需要专用结果视图的工具将展示声明和结果载荷分别保存在已有 `MessagePart.attributes` 的 `display`、`result_data` 属性中。这些字段使用 JSON；XML 传输时按属性转义，读取时只解码一次。没有新增存储文件、表或数据库迁移。通用文本视图不额外附加重复的结构化结果。完整结构化结果仅在现有单条工具结果预算内附加，超过预算时仍保留原有的有界文本结果。多步骤结果以合并文本保存。

结果通过 `ToolResultData::toModelString()` 生成模型文本，`toString()` 生成用户展示文本。通用 `PresentedResultData` 分别持有展示结果和模型结果，两者都可包含文本或结构化数据。原生 `FileApplyResultData` 保留已有业务结构，通过结果适配生成操作摘要和 diff 展示。摘要在展示文本截断前生成。一次调用产生多个步骤时，各步骤先分别投影再合并，保留普通输出和步骤错误。

第三方包可返回显式展示契约，工具名称和业务字段不参与推断：

```ts
import type { ToolResultPresentation } from "../../../types/core";

const result: ToolResultPresentation<{ diffContent: string }, string> = {
  $operitToolResult: {
    display: { diffContent: "+full source for the user" },
    model: "Updated file: +1 line"
  }
};
return result; // complete(result) uses the same contract.
```

`display` 按工具已有的 `resultRenderer` 渲染；`model` 是包明确提供的模型值，可以是字符串、对象、数组、标量或 null。显式空字符串保持为空，不使用展示内容填充。包间调用转交该结果时保留公开契约，`package_proxy` 和 `package_wait` 也保留两种结果，回执元数据分别加入两种投影。

旧包直接返回单个值、对象或数组以及 `complete(value)` 的行为保持不变。普通对象里的 `display`、`model`、`__type` 字段仍是业务字段。`$operitToolResult` 是新协议的保留标记；使用该标记时必须给出完整的 `display`、`model`，非法契约明确报错。原生结果自身的 `__type` 只用于内部序列化；转换为 SDK 业务值和模型回执时移除，不删除包自己的业务字段。

两种文本不同时，展示消息在 `model_content` 属性中携带独立的、有界模型正文（包含错误封装），并随 `MessagePart.attributes` 存储。历史序列化使用该正文；`MessagePart.content` 仍保留用户展示文本。旧版 `apply_file`、`write_file`、`edit_file`、`create_file` 历史中的显式 `file-request-content` 也按原协议提取，普通读文件结果不参与这项转换。

提供给模型的工具结果及历史序列化排除 `display`、`result_data` 和 `model_content` 属性，避免展示声明和重复结果进入模型上下文。包代理和包执行协议不变；未声明展示契约的插件 JSON 按原内容处理。工具分组继续使用既有 `ThinkToolsXmlNodeGrouper` / `ToolCallResultMergeRender` 机制。
