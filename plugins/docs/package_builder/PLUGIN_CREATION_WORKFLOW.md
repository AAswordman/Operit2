# 当前版本插件创作流程

本说明适用于通过 PackageBuilder 开发当前 Operit 的脚本包和 ToolPkg。接口以本 Skill 的 `types/` 为准；教程中的 `<dev_root>`、`<artifact_host_path>`、`<package_id>`、`<runtime_package_name>` 和 `<tool_name>` 是需要用真实值替换的占位符。

## ToolPkg API 版本与跨平台要求

Operit2 的 ToolPkg API 支持版本为 `2.0.0`；对 `1.0.0` 和 `1.0.1` 的加载支持并不完整。Operit1 完整支持 ToolPkg API `1.0.0` 和 `1.0.1`，这两个版本主要面向 Android，是旧版 API 形式。

为 Operit2 新建 ToolPkg 时，在 manifest 中显式声明 `"api_version": "2.0.0"`，并使用 PackageBuilder 随包携带的当前版本 `types/`。解析器接受旧版本号不等于完整实现旧接口；旧包需要逐项验证所用能力，不能仅凭导入成功宣称兼容。迁移旧包需要同步检查接口、路径和平台语义，不能只修改版本号。

ToolPkg API `2.0.0` 面向多平台开发，基本所有公共接口都通过统一 host 能力提供跨平台兼容。常规功能直接使用当前版本接口即可，不必为每个平台重复实现。作者仍需注重跨平台兼容性：遇到平台特异接口时，明确其适用范围，考虑并验证其他平台的安装、界面与核心功能；不能把某个平台的专用能力当作所有平台都具备的接口。

`schema_version` 表示清单格式版本，`api_version` 表示 ToolPkg API 契约，`version` 表示插件自身的发布版本，三者不能混用。

## 1. 创作入口实际做什么

包管理中的“快速创作你的插件”会安装 PackageBuilder、将它设为 AI 可见，并启用 operit_editor。准备完成后，需求会作为聊天草稿填入输入框，用户发送后才开始与 AI 协作开发。这个入口不自动生成源码、不自动配置编译器，也不自动安装或发布成品。

operit_editor 的 `operit_editor` 工具实际调用 `Tools.SoftwareSettings.exec(args)`，与系统工具 `execute_cli_command` 使用同一 core command 执行入口，返回真实命令结果并传播执行错误。传入 `args` CLI 字符串数组（也接受数组的 JSON 字符串），不含 `operit2` 可执行文件名。自然语言 `query` 已移除，不会被转换成编辑动作。Core command 与终端 shell 是不同的执行入口，不要假定终端已安装 `operit2` 命令。

调用 `operit_editor:operit_editor` 的参数示例：

```json
{"args": []}
{"args": ["package", "list"]}
{"args": ["skill", "show", "PackageBuilder"]}
{"args": ["package", "exec", "<runtime_package_name>:<tool_name>", "{}"]}
```

空数组从实际执行入口读取帮助；需要包命令帮助时传入 `["package", "help"]`。先读取真实状态，修改用户配置、启停包或 MCP、删除资源前取得用户确认，执行后重新读取状态核验；不能把命令已调用当作修改成功。

### 已安装的 Skill 如何更新资料

当前创作准备流程会复用已经安装的 PackageBuilder，不会自动覆盖它的附件。更新应用资源不会同步改写用户已安装的 Skill。

要使用当前版本的内置资料，先核对 `["skill", "show", "PackageBuilder"]` 返回的目录并备份用户修改；获得用户确认后执行：

```json
["skill", "delete", "PackageBuilder"]
["skill", "load", "PackageBuilder"]
["skill", "visible", "PackageBuilder", "true"]
["skill", "show", "PackageBuilder"]
```

逐项核验结果及 references/types/examples 文件，不删除其他 Skill；任一步失败停止并报告。已有安装不应被描述成“已经自动更新为当前资料”。

## 2. 确定真实开发目录

1. 调用 `execute_cli_command`，参数数组为 `["skill", "show", "PackageBuilder"]`，读取 Skill 的真实目录与附件位置。
2. 查看系统工具 `get_terminal_info`；脚本内对应 `Tools.System.terminal.info()`。根据 host 返回的 `terminalType` 和可用终端信息使用正确的 shell 语法。该接口不返回下载目录或开发目录。
3. 在已选定的终端会话中查询实际工作目录及工具版本，确定用户同意使用的可写目录，记为 `<dev_root>`。确认源码、类型目录与成品所在位置均可通过相关文件工具访问。路径或工具不可用时，报告具体原因并停止，不猜目录、不改用另一套执行入口。
4. `Tools.Files` 接收 VFS 路径；终端和 `package import` 需要各自 host 可读的路径。VFS 路径、终端路径与 FileSystemHost 路径不能仅凭相同字符串认定指向同一文件。核对真实文件内容与路径映射后再使用。

不要把某个平台的下载目录、应用私有目录或开发者电脑目录写成所有用户固定使用的路径。`["package", "dir"]` 返回的是安装存储目录，不是源码开发目录；`host paths` 和 `host capabilities` 当前并未通过 core command 暴露。

开发目录组织为：

```text
<dev_root>/
  types/                 # 从已安装的 PackageBuilder/types 完整复制
  <package_id>/
    package.json         # TypeScript 开发依赖与构建命令
    tsconfig.json
    src/
    dist/
    manifest.json        # ToolPkg 清单；单文件脚本包不需要
  artifacts/             # 成品位于源码目录之外，避免打包包含自身
```

包 id 确定后保持不变。继续开发时先定位原始源码；安装成品不能被当作完整源码项目。确认包内的 `.ts`、`tsconfig.json`、资源与 `dist/` 是否齐全，说明缺失项，不凭空声称已经恢复项目。

## 3. 编写与构建

- 完整复制 Skill 的 `types/`，不从旧仓库或云端下载另一版声明。项目通过 `../types` 引用声明；位于 `src/` 的三斜线引用是 `../../types/index.d.ts`。
- TypeScript 输出可读的 CommonJS JavaScript。Compose UI 保留 `ctx.UI.*` 界面声明和回调，由运行时完成 Compose 编译与缓存；交付文件供用户直接编辑，不生成 Compose 内部预编译产物。按实际 shell 执行项目已声明的构建命令，检查退出码、超时状态和真实输出文件。没有可用编译器时说明环境未准备完成。
- 网络、文件与终端操作使用 `Tools.Network`、`Tools.Files`、`Tools.System.terminal` 及其 host API。不要用平台分支实现另一套插件运行链路。声明文件的存在不代表当前 host 实现了所有能力。
- 单文件脚本包保留 `METADATA` 与导出函数；ToolPkg 在归档根目录放置 manifest，并按清单路径包含编译后的入口、子包、UI、资源与二次开发源码。
- 用文件工具 `Tools.Files.zip(sourceVfsPath, artifactVfsPath, false)` 可创建不含外层目录的归档。成品扩展名为 `.toolpkg`，manifest 必须位于 ZIP 根目录。核验成品存在、内容完整，以及 runtime host 可读的 `<artifact_host_path>`。

## 4. 首次安装与测试

以下每行都是一次 operit_editor 工具的 `args` 参数数组，也可用于系统工具 `execute_cli_command`：

```json
["package", "list"]
["package", "import", "<artifact_host_path>"]
["package", "enable", "<package_id>"]
["package", "list"]
["package", "show", "<runtime_package_name>"]
["package", "exec", "<runtime_package_name>:<tool_name>", "{}"]
["log", "package"]
```

- 导入结果给出 `packageName`、`packageFormat` 和 `storedPath`。以实际解析结果确认身份，不根据文件名猜包 id。
- ToolPkg 容器 id 与可执行子包名不同。从 `package list` 和 `package show` 读取实际工具名；只注册 UI、hook 或 provider 的包不一定有可执行工具。
- 启用后重新读取 `package list` 的真实 `enabled` 状态；不能把启用命令自身的文字或请求参数当作状态证明。
- `package exec` 会激活对应工具包并执行导出工具。根据该工具声明填写 JSON 参数，检查返回内容；UI、hook、provider 必须在对应应用场景验证。
- 调试小片段时，将它写成有 `METADATA` 和导出函数的独立测试包，按导入、启用、执行、查看日志的流程测试。operit_editor 不提供脚本直跑工具。
- 每一步出现错误即停止并定位原因。不要根据“命令已调用”宣称安装、启用或测试成功。

## 5. 同 ID 的修改与重新安装

当前 `package import` 拒绝重复的已注册包名，不提供覆盖安装或自动热更新。更新步骤是用户明确授权后的替换操作，不是失败后的自动处理：

1. 在删除已安装版本之前完成新版本构建，核验归档，并在开发目录保留源码及所需旧版本成品。
2. 用 `package list` 和 `package show` 记录容器、子包的真实名称及启用状态；向用户说明删除与重新安装的影响并获得确认。
3. 调用 `["package", "delete", "<package_id>"]`，确认删除成功。只对本次要更新的外部包执行该操作，不删除其他包。
4. 调用 `["package", "import", "<artifact_host_path>"]`，确认新版本导入成功。导入失败时保留错误并停止，不自动改变包 id 或执行其他安装路径。
5. 按用户确认的配置逐项恢复容器及子包的启用状态，再读取列表核验，并重新测试工具、UI 与 hook。

不要宣称包管理已经具有一键继续开发、合并开发或覆盖安装功能。创作入口目前接入的是新建需求草稿；继续开发需要明确定位原项目并按上述流程处理。

## 6. 发布与交付

本地安装、执行和界面测试完成后，用户可在 Artifact 市场的发布页面选择已安装的本地插件，填写资料和版本：

- 直接发布本地插件：页面处理当前成品，在当前 GitHub 账号的 OperitForge 仓库保存 Release 资产并登记市场条目。
- 引用 GitHub Release 资产：选择自己仓库的 Release 与资产；页面核对所选资产与当前本地成品，并提交登记。

发布需要登录、用户确认及真实服务响应，不是创作入口自动完成的动作。交付时分别列出源码目录、成品路径、包 id、安装与启用状态、已执行的测试和结果，以及发布状态。未测试或未发布的环节要明确标注。
