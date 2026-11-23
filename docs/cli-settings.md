# CLI 设置与界面能力对照

CLI 设置命令调用 Flutter 所用的 Core 服务和配置管理器。以下命令均支持
`operit2 cli --json …`，输出结构化结果；错误返回非零退出码。运行中的 CLI
Core/daemon 接收终端、主题和 TTS 命令，避免为同一身份另开一个配置运行时。
CLI 身份与 Flutter 身份各有自己的 CoreNode 配置；命令作用于当前 CLI 身份，
其中终端路径与 rootfs 属于设备，不作为 Space 配置同步。

## 本次补齐的入口

| 界面功能 | CLI 命令 |
| --- | --- |
| 终端类型、实例与支持字段 | `terminal info`、`terminal support`、`terminal profiles` |
| 添加、编辑、默认终端、删除 | `terminal add`、`update`、`default`、`delete`、`save` |
| ARM64 发行版目录、安装与取消 | `terminal distributions`、`install`、`installs`、`cancel` |
| 终端会话管理 | `terminal sessions`、`open`、`input`、`screen`、`close` |
| 主题创建、编辑、选择及外观 | `theme list/show/active/create/update/rename/delete/apply`、`appearance`、`appearance-save`、`appearance-patch`、`custom` |
| 语言、回车发送、长文本粘贴 | `prefs language`、`prefs input` |
| 总工具开关、用户偏好说明、功能及工具提示可见性 | `prefs tools`、`preference-description`、`feature`、`tool-visible` |
| 插件 pre-hook 超时 | `prefs pre-hook-timeout` |
| 完整提供商配置及 API key 池 | `model provider-save`、`provider-delete`、`provider-key-failures-clear` |
| 模型完整配置、删除及规格覆盖重置 | `model profile-save`、`delete`、`spec-reset`、`thinking-spec` |
| 提供商单字段编辑 | `model provider-set` |
| 工具调用、图片／音频／视频直传 | `model capabilities-show`、`capabilities-set` |
| 结构化工具调用、Claude 一小时缓存 | `model request-show`、`request-set` |
| 内置工具与模型参数单项勾选 | `model builtin-tools-show`、`builtin-tool-set`、`param-set` |
| 模型配置原始值读取、分区写入 | `model profile-show`、`profile-set` |
| 功能模型跟随聊天 | `model function-follow-chat [function-type]` |
| 模型连接及凭据测试 | `model test`、`provider-test-key` |
| 提供商配置备份导入导出 | `model providers-export`、`providers-import` |
| Codex 会话状态与退出 | `model codex-status`、`codex-logout` |
| 本地模型文件导入、测试与释放 | `local-models file-engines`、`file-import`、`test`、`unload` |
| TTS 提供商和音色 | `tts provider-list`、`voices`、`voice-add`、`voice-custom` |
| TTS 完整 HTTP 请求、请求头、响应流水线 | `tts config create-json/save` |
| STT 完整配置 | `stt config create-json/save` |
| 未绑定工作区清理 | `workspace summary`、`delete-unbound --yes` |
| 当前设备用户名 | `link space user-name` |
| GitHub 登录状态与退出 | `market auth status/logout` |

原有模型参数、内置工具、上下文与压缩配置，MCP、插件与技能管理，
存储迁移、备份恢复、用量、空间成员、角色、权限、配对、监听和网络策略入口继续使用。
`model function-reset` 表示重置到默认绑定，`function-follow-chat` 才表示跟随聊天模型。

## AI 提供商与模型勾选项

这些选项属于提供商下的具体模型，需要同时传入 provider ID 和 model ID；
先用 `model provider-list`、`model list` 查询。查看命令同时返回生效值和显式覆盖值，
`true` / `false` 固定该勾选项，`auto` 只删除这一项的覆盖，恢复现有目录／模型列表的继承规则。
修改其他勾选项、参数、上下文和凭据不会受影响。`prefs tools on/off` 是全局工具开关，
模型的 `tool-call` 和请求的 `structured-tools` 分别控制模型能力与工具请求格式。

```sh
operit2 cli --json model capabilities-show <provider-id> <model-id>
operit2 cli model capabilities-set <provider-id> <model-id> tool-call true
operit2 cli model capabilities-set <provider-id> <model-id> direct-image true
operit2 cli model capabilities-set <provider-id> <model-id> direct-audio false
operit2 cli model capabilities-set <provider-id> <model-id> direct-video auto
operit2 cli --json model request-show <provider-id> <model-id>
operit2 cli model request-set <provider-id> <model-id> structured-tools true
operit2 cli model request-set <provider-id> <model-id> claude-1h-prompt-cache true
operit2 cli --json model builtin-tools-show <provider-id> <model-id>
operit2 cli model builtin-tool-set <provider-id> <model-id> WebSearch true
operit2 cli model param-set <provider-id> <model-id> temperature false
operit2 cli model param-set <provider-id> <model-id> temperature true 0.7
```

内置工具使用已配置列表中的 `WebSearch` / `CodeExecution` / `UrlContext`，只修改
`enabled`，保留请求格式、排他规则和配置；不存在或重复的工具类型会明确报错。
`param-set` 保留未提供的当前值；提供新值时使用 JSON，字符串需要 JSON 引号，
也可传 `@<VFS 文件路径>`。参数 ID 以 `model params` 的结果为准。

提供商单字段编辑支持 `name`、`endpoint`、`api-key`、`custom-headers`、
`multiple-api-keys`、`api-key-pool`、`request-limit`、`max-concurrent`。
布尔值使用 `true/false`，请求限制为非负整数，0 表示不限制；请求头是字符串值的 JSON 对象，
key 池使用现有 `ApiKeyInfo[]` 完整结构。字段值均支持 `@<VFS 文件路径>`，输出沿用
`provider-show` 的凭据脱敏格式。

```sh
operit2 cli model provider-set <provider-id> request-limit 60
operit2 cli model provider-set <provider-id> max-concurrent 4
operit2 cli model provider-set <provider-id> multiple-api-keys true
operit2 cli model provider-set <provider-id> custom-headers @/operitapp/workspaces/demo/headers.json
operit2 cli --json model profile-show <provider-id> <model-id>
operit2 cli model profile-set <provider-id> <model-id> local-runtime @/operitapp/workspaces/demo/runtime.json
operit2 cli model profile-set <provider-id> <model-id> pricing null
```

`profile-show` 返回可直接供 `profile-save` 使用的原始 `ModelProfile`；`model show`
继续返回解析后的运行配置。`profile-set` 替换指定分区，支持 `pricing`、`context`、
`capabilities`、`builtin-tools`、`request`、`thinking-spec`、`thinking-level`、
`parameters`、`summary`、`local-runtime`。JSON 类型与原始 profile 中对应字段一致，
`capabilities` / `request` 使用可空的逐项覆盖对象；可选分区传 `null` 表示删除覆盖。
分区对象整体替换，不进行递归合并；未指定的分区保持原值。
`parameters` 和 `builtin-tools` 原有的完整列表写入命令也支持 `@<VFS 文件路径>`。

## 终端实例

先读取当前宿主支持的类型和字段：

```sh
operit2 cli --json terminal support
operit2 cli --json terminal profiles
```

Windows 例如可添加两个独立配置。将各个 JSON 保存到当前运行时可访问的文件中，
通过 `@<VFS 文件路径>` 传入 `terminal add`：

```json
{
  "id": "powershell7",
  "name": "PowerShell 7",
  "terminal": "native",
  "terminalType": "powershell",
  "options": {
    "executable": "C:/Program Files/PowerShell/7/pwsh.exe",
    "arguments": "[\"-NoLogo\"]"
  }
}
```

```json
{
  "id": "wsl-ubuntu",
  "name": "WSL Ubuntu",
  "terminal": "native",
  "terminalType": "bash",
  "options": { "wslDistribution": "Ubuntu-24.04" }
}
```

```sh
operit2 cli terminal add @/operitapp/workspaces/demo/powershell7.json
operit2 cli terminal add @/operitapp/workspaces/demo/wsl-ubuntu.json
operit2 cli terminal default wsl-ubuntu
operit2 cli terminal update @/operitapp/workspaces/demo/wsl-ubuntu.json
operit2 cli terminal delete powershell7
```

WSL 发行版须在运行 Core 的 Windows 账户下注册，`terminal support` 返回已发现的
发行版名称。只填写 `wslDistribution` 时，宿主使用系统的 `wsl.exe`。
只能使用 `terminal support` 返回的实现及字段。`add` 不覆盖已有 ID，`update` 不创建
未知 ID。删除默认实例前须先选择另一个默认实例；删除配置保留 rootfs 和已有会话。

`terminal save <JSON 或 @VFS文件>` 接收 `terminal profiles` 返回的完整结构，必须保留
其 `revision`。发生并发修改时保存返回冲突，重新读取并明确合并后才能再保存。

支持 rootfs 安装的 ARM64 宿主可使用以下命令；桌面宿主按自身能力返回目录：

```sh
operit2 cli --json terminal distributions
operit2 cli terminal install ubuntu-24_04 "Ubuntu 开发"
operit2 cli --json terminal installs
operit2 cli terminal cancel <install-id>
```

`install` 等待完成后输出安装结果，失败或取消返回错误，保证单次 CLI 进程存活期间
下载能完成。使用常驻 Core 时，可从另一个命令查询或取消同一个任务。发行版 ID 使用
目录中的实际值。每次安装创建独立实例，只有 Alpine 随软件内置。

会话跨命令使用时，先启动常驻 Core（`operit2 daemon start`）：

```sh
operit2 cli terminal open "开发终端" /operitapp/workspaces/demo wsl-ubuntu
operit2 cli terminal input <session-id> @/operitapp/workspaces/demo/commands.txt
operit2 cli --json terminal screen <session-id>
operit2 cli terminal close <session-id>
```

省略 `open` 最后的配置 ID 使用默认配置；`input` 按原样发送文本，执行命令所需的
换行也应包含在输入文件中。

## 其他设置示例

```sh
operit2 cli prefs language zh-CN
operit2 cli prefs input enter-send auto
operit2 cli prefs input long-paste on 5000
operit2 cli prefs pre-hook-timeout 60
operit2 cli prefs tools on
operit2 cli prefs preference-description off
operit2 cli prefs feature <feature-key> on
operit2 cli prefs tool-visible <tool-name> off
operit2 cli --json prefs show
operit2 cli model function-follow-chat SUMMARY
operit2 cli model spec-reset <provider-id> <model-id>
operit2 cli model test <provider-id> <model-id>
operit2 cli model provider-save @/operitapp/workspaces/demo/provider.json
operit2 cli model profile-save <provider-id> @/operitapp/workspaces/demo/model.json
operit2 cli local-models file-import LlamaCpp "本地模型" /operitapp/workspaces/demo/model.gguf
operit2 cli --json workspace summary
operit2 cli workspace delete-unbound --yes <workspace-name>
```

`enter-send auto` 删除显式覆盖，由界面按布局决定默认行为；长粘贴阈值为
1000–20000。`file-import` 按块读取 VFS 文件，MNN 的多个文件可以一次传入。
`local-models test` 的最后一个参数是完整的 `LocalModelRuntimeSettings` JSON，
本地推理设置也可随 `model profile-save` 持久化。

主题 `create/update/appearance-save/custom` 使用与 Flutter `ThemePreferenceSnapshot`
一致的完整快照（驼峰字段名），可以从 `theme show <id>` 的 `snapshot` 中取得；
`theme appearance` 查看当前底层外观偏好（下划线字段名）。
`theme appearance-patch <initial-snapshot> <values> [remove-keys]` 接收明确的完整初始
快照、底层偏好键值对象和可选的删除键数组，通过管理器原子更新当前外观及活跃主题。
例如 `values` 可为 `{"theme_mode":"dark"}`，删除数组可为 `["background_image_uri"]`。
不会通过通用文件写入绕过主题管理器。

TTS 保留已有的 `config create/create-local/update` 和 `synthesize --config … --text …`
用法，完整 JSON 写入补足请求头和响应流水线等字段。STT 也保留现有命令。
普通 TTS 文本输出隐藏 API key，显式 JSON 输出可用于完整配置导出；提供商
`providers-export` 的备份同样包含凭据，应保存到用户自己的文件中。

界面内的文件选择、布局预览、设备权限授权等交互由客户端或操作系统处理。
CLI 可设置其对应配置值，设备授权仍走宿主已有的授权流程。
