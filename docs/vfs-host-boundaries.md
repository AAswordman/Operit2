# VFS 与 Host 边界

依赖方向为：

```text
工具、Provider、Runtime、CLI
             │
             ▼
     operit-host-api（装配）
        │            │
        ▼            ▼
   operit-vfs ──► operit-host-contract
                      ▲
                      │
              平台后端实现
```

`operit-host-contract` 定义平台能力、文件引用契约与存储命名空间，不依赖 VFS
实现或业务 crate。`operit-vfs` 实现挂载目录、路径映射和文件操作，仅依赖 host
契约。`operit-host-api` 负责 `HostManager` 装配与兼容导出，不再拥有契约定义。
本地文件与存储后端直接依赖契约层，避免通过装配层反向引入 VFS。

## 文件入口

业务代码从 `context.vfs()` 获取所属运行实例的文件系统，通过 VFS 地址读写。
不得重新收集 `runtimeRootDir`、`workspaceRootDir` 与底层文件 host 来装配另一套
映射。

`PathMapper` 是 VFS crate 的私有模块，外部无法导入、构造或取得它。业务使用
`VfsPath` 进行虚拟地址拼接和校验，通过所属 VFS 的操作接口或 `VfsFile` 完成
文件访问。纯虚拟地址运算不会返回后端定位符；原生地址转换留在 VFS 和 Host
消费边界内。编译失败文档测试和依赖检查共同约束这一点。

工具调用额外经过 `FileToolRouting`：它读取当前工作区或聊天输出目录，解析相对
路径，并处理 `节点:/虚拟路径`。节点选择、远程工具派发、工具结果重写留在工具层，
基础 VFS 不依赖聊天、节点路由和工具执行器。跨节点请求在目标节点自己的 VFS
中解析；目标不可用或调用失败时直接返回错误。

`install_app` 与打开、分享工具使用同一个路径准备和路由入口。安装仍保留外部
写操作的权限策略，异步派发不会改变审批边界。

## 文件消费契约

`VfsFile` 绑定虚拟地址和所属 VFS，字段保持私有。序列化时只保存 `VfsPath`；
反序列化检查绝对地址、非法段与空字符。文件引用本身不序列化，接收方使用自己的
VFS 重新创建引用，不能借序列化传递原运行实例的存储权限。

- 安装和 OCR host 接收 `FileSource`，可读取内容、范围或虚拟元数据。
- 单文件 HTTP 下载接收 `FileTarget`；原生 HTTP 实现自己取得平台目标，保留
  流式下载和完整后发布的语义。
- 底层文件后端仍接收原生路径或不透明资源定位符。该接口属于 VFS 与平台适配层。
- 打开、分享、普通读写、工作区、备份和媒体注册通过同一 VFS 派发。
- 消息附件的存在检查与内容读取使用所属运行实例的 VFS，保留虚拟文件身份。
- 跨设备传输通过 VFS 分块读写、校验长度与 SHA-256 后发布。接收会话保留所属
  VFS，公开结果返回虚拟地址。Android 文档挂载的分块追加由 Android host 实现。

`nativePath()` 是平台互操作入口。资源挂载没有原生路径时显式失败；平台若需要
使用这类文件，应实现对应资源读取或明确的临时文件生命周期，不能在核心中捕获
失败后改走另一个后端。VFS 文件引用不替代工具权限检查或平台授权。

## 核心与平台的边界

Core 的音频播放、推理、进程工作目录和插件文件消费使用所属 VFS 的文件引用。
本地推理的 FFI、引擎命令和原生构建位于 hosts/common/operit-host-local-inference；
Core 仅选择模型、构造虚拟资源和派发 Host 请求。系统语音合成向 Core 返回音频字节，
临时原生文件在平台 Host 内读取并清理。终端的工作目录转换封装在 VFS 与终端 Host
之间，业务不取得通用的原生路径解析器。

RuntimeStorePaths 是无状态的虚拟命名空间描述，不接受物理 root；配置和 Provider
构造也不再接受 root。物理存储根只在平台启动装配与挂载注册中配置。CLI 将调用者
当前目录转成虚拟地址传给 Core，并随本地或 daemon 命令执行传递。

resolvePath 仅在 VFS crate 内可见。编译期 AST 检查阻止 Core 服务直接使用文件
Host、提取 nativePath、读取物理存储根或调用原生文件和进程 I/O。检查不为业务
互操作保留豁免；测试夹具与构建代码按自身用途检查。

## 存储位置管理

存储设置通过独立的 `storageLocations()` 管理接口查询实际落点。每一项同时包含
稳定的 `vfsPath`（例如 `/operitapp/data`）、指向该目录的 `locationVfsPath`
（例如 `/virtual/windows/C/...`）及供用户识别磁盘位置的 `displayPath`。
`displayPath` 仅用于设置展示；普通文件、插件和工具接口使用虚拟地址。

这两个虚拟地址访问相同的文件。目录包含判断按底层资源身份比较，避免通过第二个
地址绕过迁移的自包含检查。CLI 存储迁移接受用户选择的位置，先转换成 VFS 地址
完成复制，再由 CLI 平台配置接口解析所选目录、保存实际根目录配置；后续启动时
应用别名指向新位置。`storage paths` 因而能显示实际存储目录，业务命名空间保持稳定。

存储选择使用 `VfsPath::storageLocationPath()`，不会套用插件文件的旧共享目录别名。
例如用户选中 `/storage/emulated/0/Download/Operit`，对应地址为
`/virtual/android/sdcard/Download/Operit`。Flutter 设置继续通过平台存储管理接口
显示、选择实际目录，提交迁移时才进入 VFS。

## 验证

基础层测试覆盖同名虚拟地址在不同运行实例中隔离、范围读取、虚拟元数据、路径
反序列化校验、资源引用读取和挂载目录持久化。工具测试覆盖相对安装路径、本机
节点前缀、远程派发与失败不触发本机安装。边界检查阻止普通业务代码直接取得
原生定位符。Android 系统安装器、打开和分享面板仍需设备验收。

2026-11-11 在 Windows 上完成以下验证：

- CLI 实际构建成功；`node tools/tests/windows_vfs_cli_acceptance.mjs` 的 52 项命令验收通过。
  脚本创建独立配置和身份目录，核对返回路径与实际文件字节，覆盖读写、复制移动、
  压缩解压、HTTP 下载、工作区进程目录、ToolPkg/Skill/MCP 压缩包安装、MCP 覆盖安装、
  存储迁移、自包含拒绝及重启后的别名等价。安装、打开、分享的 CLI 检查仅覆盖缺失
  输入被拒绝，没有进行真实软件安装或系统面板交互。每次运行输出独立 results.json。
- daemon 实际启动后，相对路径写入和读取使用调用者目录，核对实际字节后停止服务。
- VFS 单元测试 38 项、拒绝外部访问 Mapper 的编译失败文档测试 3 项、本地模型
  单元测试 25 项、命令层单元测试 35 项、Node 架构与平台契约检查 52 项通过。
- Flutter Windows Rust bridge 的 `cargo check --lib` 通过；目录选择的 8 项定向
  Flutter 测试通过。完整目录选择测试在 onboarding 身份初始化用例中未完成，已停止
  该测试进程；定向测试不包含这项启动流程。

更大范围测试未全绿：Runtime 为 345 项通过、4 项失败、1 项忽略，失败涉及
PreferenceSpaceMerge 的原始操作与 schemaVersion 断言，以及 EdgeChatProjection
的历史标记和完成检查断言。Tools 为 213 项通过、1 项失败、2 项忽略，失败发生在
configuration_public_api_deadline_isolates_broken_scripts 结束时删除测试目录，
Windows 报 SQLite 文件仍被占用。没有通过吞掉错误或忽略失败断言使这些检查通过。

后续补充验证：附件来源集成测试 1 项、文件选择集成测试 4 项通过；
`workspace_binding_test` 在创建聊天时因 `Chat lifecycle runtime is not registered`
失败，该项尚未通过。Windows Flutter 的 `fvm flutter build windows --debug` 已完整
构建成功；Bash 安装规则改为在应用安装目录配置完成后声明，修复目标目录表达式未
展开而导致的 CMake INSTALL 失败。Bash 打包的 3 项测试通过。

## 尚待收口的边界

终端链路仍通过公开的 `VisualFileSystem::terminalDirectory()` 向 Core 返回终端目录
字符串，当前共有 4 个业务文件、5 处调用。后续应由终端 Host 接收绑定所属 VFS 的
目录能力，在内部完成解析、进入目录和执行，删除这个对外解析入口，并补充边界检查。
终端展示与 shell 命令本身继续使用对应终端的路径语义。

聊天附件入口仍在 `ChatServiceCore` 内解析 `file://`，且包含平台分支；随后导入过程
会先转换为 VFS 地址并通过 VFS 访问文件。该入口解析应移至统一资源入口，避免聊天
业务承担平台路径处理。以上两项尚未完成，不能将本次迁移描述为所有边界已经收口。

## v1 语音插件

旧插件的 `/storage/emulated/0/...` 与 `/sdcard/...` 在文件工具和资源读取入口都先按同一份插件存储别名解析。文件生成、媒体注册和 WebView 的 `file://` 音频读取因此访问同一文件；显式 `/virtual/android/sdcard/...` 仍表示平台共享存储挂载。

v1 的 `Tools.FFmpeg.execute/info/convert` 与历史 `ffmpeg:ffmpeg_execute` 工具调用共用内部兼容实现。虚拟输入通过所属 VFS 读取，暂存到当前插件选择的运行环境；FFmpeg 成功后才把输出写回 VFS，并在成功和失败时清理暂存目录。插件收到的路径和输出诊断保留原地址。转换参数按独立参数传递，兼容已发布语音插件探测时长所用的 `2>&1 || true` 后缀。

Android、iOS、OHOS 的 Alpine 运行环境首次使用时检查 FFmpeg，缺少时通过现有 APK 包管理器安装；安装失败保留实际错误且允许重试。本机运行环境使用 PATH 中已有的 FFmpeg。

可用未修改的市场插件包进行可选集成验证，`OPERIT_TEST_FFMPEG` 指向本机 FFmpeg 可执行文件，另外两个变量指向解压后的插件目录：

```sh
OPERIT_TEST_FFMPEG=/absolute/path/to/ffmpeg \
OPERIT_VOICE_BAR_FIXTURE=/absolute/path/to/com.operit.voice_bar \
OPERIT_VOICEBAR_FIXTURE=/absolute/path/to/com.gardencottage.voicebar \
node --test tools/tests/toolpkg_v1_runtime.test.mjs tools/tests/toolpkg_v1_adapters.test.mjs
```

测试使用合成音频代替服务商返回值，验证真实转码、时长、缓存和系统朗读调用，不使用服务商凭据。
