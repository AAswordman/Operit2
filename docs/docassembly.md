# Operit2 Doc Assembly

本文档汇总 Operit2 当前工程的产品形态、代码分层、运行链路、插件体系、构建发布约束与开发规则，用作项目接近完成阶段的总装文档。

## Project Goal

Operit2 是一个跨平台 AI 助手运行时项目。工程同时提供 Flutter App、CLI/TUI、Rust runtime、平台 Host、插件/ToolPkg、工作区模板、发布工具与 Android runtime 资源。

核心目标：

```text
统一 runtime
统一 host trait
统一插件协议
统一工具调用与结果表达
统一桌面、移动端、Web、CLI 的运行边界
```

Rust runtime 的目标是复刻原 Kotlin Operit runtime 的结构、命名、模块顺序与行为表达。迁移工作以 Kotlin 源实现为基准，Rust 文件保留 Kotlin 风格的文件名与字段命名。

## Repository Layout

```text
apps/
  cli/                 Operit2 CLI/TUI
  flutter/             Flutter App 与 native bridge
  server/              服务端应用入口

core/
  crates/
    operit-runtime/    核心 runtime
    operit-host-api/   Host trait 与跨平台数据结构
    operit-link/       Runtime 与代理通信协议
    operit-core-proxy/ Core proxy 与代码生成
    operit-command-core/
                         CLI command 共享实现
    operit-store/      存储抽象与 SQLite/ObjectBox 风格实现

hosts/
  android/             Android host
  ios/                 iOS host
  linux/               Linux native host
  server/              Server host
  web/                 Web/WASM host
  windows/             Windows native host

plugins/
  buildin/             内置 ToolPkg 源码
  docs/                插件作者文档
  examples/            示例 ToolPkg
  tools/               插件同步与开发工具
  types/               TypeScript 声明

tools/
  android-runtime/     Android runtime 构建资源脚本
  release/             全量发布脚本与 release 规范
```

## Core Crates

### operit-runtime

`operit-runtime` 是核心业务层，导出 AI service、消息管理、工具系统、插件系统、数据模型、仓库、偏好设置、工作区与工具函数。

入口：

```text
core/crates/operit-runtime/src/lib.rs
```

主要模块：

```text
api/       chat runtime、EnhancedAIService、LLM provider、conversation enhance、memory library
core/      application、chat hooks、tools、default tools、javascript、mcp、avatar
data/      model、dao、db、repository、preferences、sync、backup、mcp、skill
plugins/   builtin assets、toolpkg bridge、workflow、toolbox、registry
services/  ChatServiceCore 与核心 delegate
ui/        workspace 相关 runtime-side UI 支撑
util/      stream、parser、path、media、network、document、logging
```

### operit-host-api

`operit-host-api` 定义 runtime 调用平台能力的 trait 与数据结构。

能力分组：

```text
fs.read
fs.write
fs.search
fs.archive
web.visit
runtime.process
runtime.storage
runtime.sqlite
os.open
os.share
system.location
system.notifications.read
system.app_usage
system.app.install
system.app.uninstall
system.settings
```

Host 描述结构使用 `HostEnvironmentDescriptor`，当前包含 Android、Windows、Linux、Web 的路径说明、示例路径、能力列表与环境参数约束。

### operit-link

`operit-link` 承担 core call、object path、event stream、client/remote 通信协议。它让 CLI、App、server、host bridge 使用同一套请求与事件模型。

### operit-core-proxy

`operit-core-proxy` 负责 core proxy 代码生成与代理封装。构建脚本中包含 Rust/Dart 两侧生成逻辑，用于连接 Flutter、CLI 与 Rust core。

### operit-command-core

`operit-command-core` 封装 CLI command 实现。命令按文件拆分：

```text
approval
chat
host
market
mcp
memory
model
package
people
plugin
prefs
skill
tag
tool
update
workspace
```

### operit-store

`operit-store` 提供运行时存储路径、偏好数据、SQLite store、ObjectBox 风格 store、同步操作 store 与测试。

## Hosts

Host crate 的职责是把平台能力实现为 `operit-host-api` 中定义的 trait。各平台目录基本保持一致的 tools 结构：

```text
tools/browser
tools/fs
tools/http
tools/runtime
tools/storage
tools/system
tools/terminal
```

Windows 与 Linux host 包含 native 文件系统、HTTP、浏览器访问、runtime、storage、system、terminal 能力。

Web host 通过浏览器侧 `globalThis.__operitHost` 注入能力，Rust 端保持同一 host trait 调用形态。Web bridge 使用与 `operit-host-api` 一致的字段名，二进制数据使用 `Uint8Array`，SQLite 64 位整数以字符串传递。

Android host 与 Flutter bridge 连接 Android runtime assets、JNI libs、proot/busybox/bash/rootfs 资源。

## Apps

### Flutter App

路径：

```text
apps/flutter/app
```

职责：

```text
主聊天界面
工作区 Shell
浏览器工作区
文件预览
终端工作区
包管理与市场
设置界面
跨平台 native bridge
```

Flutter native bridge 位于：

```text
apps/flutter/native/operit-flutter-bridge
```

Android 侧资源位于：

```text
apps/flutter/app/android/app/src/main/assets/android-runtime
apps/flutter/app/android/app/src/main/jniLibs
```

### CLI/TUI

路径：

```text
apps/cli
```

二进制名：

```text
operit2
```

CLI 依赖 `operit-runtime`、`operit-command-core`、`operit-core-proxy`、`operit-link`、`operit-store`，并按平台链接 Windows 或 Linux native host。

TUI 结构：

```text
src/tui/app.rs
src/tui/render.rs
src/tui/input.rs
src/tui/markdown.rs
src/tui/commands.rs
src/tui/approval.rs
src/tui/theme.rs
```

## Plugin System

插件工作区：

```text
plugins/
```

目录语义：

```text
types/     ToolPkg 作者共享 TypeScript 声明
buildin/   编译期打包的内置插件
examples/  开发与人工测试样例
docs/      插件开发说明
tools/     插件包同步脚本
```

`buildin/` 与 `examples/` 使用同一源码布局，区别在于打包目的。

runtime 侧相关模块：

```text
core/crates/operit-runtime/src/plugins
core/crates/operit-runtime/src/core/tools/packTool
core/crates/operit-runtime/src/core/tools/javascript
```

ToolPkg 类型声明覆盖：

```text
android
chat
compose-dsl
core
cryptojs
files
jimp
material-icons
memory
network
okhttp
pako
results
software_settings
system
tool-types
toolpkg
ui
```

## Runtime Tooling

默认工具集中在：

```text
core/crates/operit-runtime/src/core/tools/defaultTool
```

标准工具模块：

```text
StandardBrowserAutomationTools
StandardChatManagerTool
StandardFileSystemTools
StandardHttpTools
StandardMemoryTools
StandardSystemOperationTools
StandardTerminalTools
StandardWebVisitTool
```

工具执行相关模块：

```text
ToolRegistration
ToolPermissionSystem
ToolExecutionLimits
ToolProgressBus
ToolResultDataClasses
AIToolHandler
AIToolHook
```

工具调用文本与 XML 标记由 `ChatMarkupRegex`、`StructuredAssistantContentParser`、stream plugins 等模块处理。

## Workspace Templates

工作区模板位于：

```text
core/crates/operit-runtime/assets/workspace_templates
```

当前模板：

```text
go
java
office
python
typescript
web
```

workspace runtime 侧模块：

```text
core/crates/operit-runtime/src/ui/features/chat/webview/workspace
```

Flutter UI 侧模块：

```text
apps/flutter/app/lib/ui/features/chat/components/workspace
```

## Build Notes

Rust workspace：

```text
core/Cargo.toml
```

CLI：

```text
apps/cli/Cargo.toml
```

Windows host：

```text
hosts/windows/Cargo.toml
```

Linux host：

```text
hosts/linux/Cargo.toml
```

Web host：

```text
hosts/web/Cargo.toml
```

Flutter App：

```text
apps/flutter/app/pubspec.yaml
```

项目约束规定 Rust 编译与运行忽略 warning。开发时关注错误、行为差异、接口契约与运行链路。

## Release Assembly

全量发布规范位于：

```text
tools/release/RELEASE_SPEC.md
```

正式 release tag：

```text
v{major}.{minor}.{patch}+{build}
```

全量更新包命名：

```text
operit2-{product}-{platform}-{arch}.{ext}
```

产品形态：

```text
app
cli
```

平台：

```text
windows
linux
macos
android
```

桌面架构：

```text
x86_64
aarch64
```

Android ABI：

```text
arm64-v8a
armeabi-v7a
x86_64
```

Android ABI 名称自身包含 `-`，asset 解析代码不得按简单横线拆分。

下载要求：

```text
Content-Length > 0
HTTP Range request returns 206
```

更新下载使用 6 线程 Range 下载。

## Kotlin To Rust Assembly Rules

Rust runtime 迁移遵守以下规则：

```text
以 Kotlin Operit runtime 为事实来源
保持 Kotlin 文件与模块顺序
保持字段命名与结构意图
保持业务行为与调用顺序
保持工具、插件、memory、chat、workspace 的边界
```

迁移文件示例：

```text
Kotlin: app/src/main/java/com/ai/assistance/operit/util/DocumentConversionUtil.kt
Rust:   core/crates/operit-runtime/src/util/DocumentConversionUtil.rs
```

当前 Rust runtime 中存在大量与 Kotlin 文件同名的 `.rs` 文件，用于逐步承接原 Kotlin runtime 行为。新增或修正文档、模型、工具、provider、repository 时，先核对 Kotlin 原实现的顺序与职责。

## Development Rules

通用规则：

```text
不改无关文件
不重排未触达模块
不替换用户已有改动
不引入与当前模块风格不一致的抽象
不把平台 host 能力写进 runtime 业务层
不让 UI 层绕过 core/runtime 契约
```

排查规则：

```text
先查真实原因
用现有模块边界定位问题
用 host trait 表达平台能力
用已有 ToolResult 与错误结构表达工具结果
用已有插件 bridge 表达 ToolPkg 扩展点
```

文档规则：

```text
发布规范写入 tools/release
插件作者资料写入 plugins/docs
项目总装与架构资料写入 docs
Host 专项资料保留在对应 hosts/* 目录
```

## Completion Checklist

项目总装阶段检查项：

```text
core crates 能独立说明职责
host trait 与 host 实现一一对应
Flutter bridge 与 Rust core 调用链清晰
CLI 与 App 使用同一 command/runtime 基础
插件类型声明与 runtime bridge 对齐
内置插件可随 runtime asset 打包
workspace templates 可被 runtime 读取
release asset 名称严格匹配 RELEASE_SPEC
Android runtime assets 与 JNI libs 已入包
Rust runtime 文件持续对齐 Kotlin Operit 原实现
```

## Current Assembly State

从当前仓库结构看，Operit2 已具备完整产品骨架：

```text
核心 runtime 已分层
平台 host 已分目录实现
Flutter App 已接入 workspace、browser、terminal、package 管理
CLI/TUI 已接入 command core 与 native host
插件 workspace 已包含 buildin、examples、types、tools
release 规范已明确 asset 命名和下载要求
Android runtime 构建脚本与资源目录已存在
```

后续文档扩展可围绕具体链路展开：

```text
chat request lifecycle
tool execution lifecycle
plugin loading lifecycle
workspace project lifecycle
host bridge lifecycle
release packaging lifecycle
```
