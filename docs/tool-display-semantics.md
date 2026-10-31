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

实时工具调用的展示声明附加在 Markdown XML 事件的 `display` 属性中。`proxy` / `package_proxy` 只按其明确的 `tool_name`、`params` 协议解析目标。`params` 新调用使用 YAML 映射文本，代码和多行正文使用 `|`、`|-` 或 `|+` 原文块；执行与展示仍兼容历史 JSON 对象及 JSON 参数文本。目标尚未完整到达时使用通用展示。

例如，原生调用的 `tool_name` 为 `files:write` 时，`params` 字符串的正文写成：

```yaml
path: /tmp/demo.py
content: |-
  import re
  pattern = r"\d+\s+\w+"
  print("hello")
```

`params` 的根节点必须是参数名到参数值的映射；数字、布尔值、空值、数组和嵌套映射保留类型。看起来像数字、布尔值或空值的字符串应加引号。`|-` 去掉末尾换行，`|` 保留一个末尾换行，`|+` 保留全部末尾换行。兼容解析直接读取已有历史记录，不需要迁移聊天数据。


需要专用结果视图的工具将展示声明和结果载荷分别保存在已有 `MessagePart.attributes` 的 `display`、`result_data` 属性中。这些字段使用 JSON；XML 传输时按属性转义，读取时只解码一次。没有新增存储文件、表或数据库迁移。通用文本视图不额外附加重复的结构化结果。完整结构化结果仅在现有单条工具结果预算内附加，超过预算时仍保留原有的有界文本结果。多步骤结果继续使用原来的合并文本回退。

提供给模型的工具结果及历史序列化排除 `display` 和 `result_data`，避免展示声明和重复结果进入模型上下文。工具分组继续使用既有 `ThinkToolsXmlNodeGrouper` / `ToolCallResultMergeRender` 机制。
