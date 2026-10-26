# 文档

按需要阅读对应主题。构建入口见 [BUILDING](../BUILDING.md)，插件开发见 [插件文档](../plugins/docs/README.md)，平台接入见 [Host 文档](../hosts/README.md)。

## 架构

| 主题 | 内容 |
| --- | --- |
| [CoreNode、Space 与 Binding](core-node-space-binding-architecture.md) | 节点、空间、执行位置和跨设备续跑 |
| [Link 与访问边界](link-access-architecture.md) | Call / Watch / Push、认证与传输职责 |
| [路由与分发](device-net-routing-distribution.md) | 链路测量、加权路由和共享 Watch |
| [Edge](edge-architecture.md) | 受限设备的成员、配置与服务边界 |
| [权限与 AI 能力](permission-access-architecture.md) | 系统权限、AI 模式、工具批准和工作区检查 |

源码结构以 [Core 领域结构](../core/crates/README.md) 和 [crate 边界](../core/CRATE_BOUNDARIES.md) 为准。

## 数据与存储

| 主题 | 内容 |
| --- | --- |
| [持久化同步](sync-architecture.md) | 接收、重放、原子提交、日志查询与内存边界 |
| [工作区文件同步](workspace-file-sync.md) | 同步范围、扫描、冲突与离线副本 |
| [VFS 挂载](vfs-mounts.md) | 工作区外部目录及文档树 |
| [附件生命周期](chat-attachment-locations.md) | 来源节点、临时文件与流式导入 |
| [插件存储](plugin-storage.md) | Preferences、对象存储和数据集事务 |
| [数据救援](data-rescue.md) | 备份、恢复与修复工具 |

## 插件与界面

| 主题 | 内容 |
| --- | --- |
| [ToolPkg 运行时](toolpkg-runtime.md) | 安装归属、自动路由、资源、超时与 XML 注册 |
| [聊天发送 API](chat-send-api.md) | 提交、控制与响应流 |
| [主题模板](theme-v1-contract.md) | ToolPkg 声明、校验与客户端配置 |
| [Theme Studio 协议](theme-studio-protocol-v1.md) | 预览握手、帧回执与导出 |
| [WebView 渲染与缩放](webview-rendering.md) | 合成、缩放、输入映射与原生视图边界 |
| [TTS 供应商](tts-providers.md) | 协议目录、音色与音频处理 |

Compose 的执行模型与二进制协议维护在 [Compose runtime](../core/crates/plugin/sdk/src/toolpkg/compose/README.md)。

## 运行与维护

| 主题 | 内容 |
| --- | --- |
| [CLI 网络控制](cli-network-control.md) | 配对、角色、成员与审批命令 |
| [TUI 无头管理](tui-headless-management.md) | 配置面板与后台 daemon |
| [版本与发布](release-versioning.md) | 版本、渠道、产物与更新 |
| [日志分级](logging-levels.md) | 日志级别、深度与查看入口 |
| [GitHub OAuth](github-oauth-broker.md) | 登录事务、回调与凭证归属 |
| [远程公告](remote-announcements.md) | 公告来源与展示 |
| [Android Root](android-root-shell.md) | 授权、执行模式与命令生命周期 |
| [Android 定时事件](android-host-event-schedules.md) | 调度、交接与失败诊断 |

## 桥接与性能

- [JavaScript / Rust 桥接](performance/javascript-rust-bridge.md)
- [Rust / Dart 共享二进制](rust-dart-shared-buffer-design.md)
- [插件发送准备](performance/plugin-send-preparation.md)

## 白皮书

- [中文](Operit2技术白皮书.pdf)
- [English](Operit2-Technical-Whitepaper.pdf)

文档维护稳定的使用方式、协议和架构边界。修复过程、阶段进度、单次跑分和测试通过清单留在提交或评审记录中；可重复执行的检查入口放在对应主题文档。
