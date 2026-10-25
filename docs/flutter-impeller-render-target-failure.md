# Impeller 渲染目标分配失败处理

## 崩溃与适用范围

TestFlight build 26 的两份报告在 iPhone10,3 / iOS 16.7.16 上，经过
`Canvas::GetLocalCoverageLimit → LogMessage::~LogMessage → KillProcess` 终止进程。
崩溃时间为 2026-10-25 09:19:46 和 10:18:21（北京时间）。

对应 SDK 为 `3.41.10-ohos-0.0.2-beta.operit.4`，维护仓库为
`AAswordman/flutter-ohos`，框架源码基线为
`71107f9b0c7303f49107294f1bc1025d42a28ac0`，引擎 ABI pin 为
`42d3d75a56efe1a2e9902f52dc8006099c45d937`。

该函数的原实现对活动通道和颜色纹理执行 `FML_CHECK`。目标创建可以在纹理
分配失败后返回空目标，调用者却直接将其压入活动通道栈，后续覆盖范围查询便
触发进程级终止。已核对发布二进制 UUID
`4C4C44A2-5555-3144-A15F-431DD0AC6446` 与崩溃报告一致。反汇编确认
相对地址 `0x3a8c60` 调用致命日志析构，返回地址 `0x3a8c64` 正好对应报告中的
函数偏移 `+712`；该分支使用源码行号 1421、断言字符串 `back_texture` 和
`Context is valid:`。因此可以确认是空纹理断言，并非空通道栈断言。
具体哪次分配失败、为何失败仍缺少设备侧日志，这两份报告不能直接判定为系统 OOM。

直接删除断言不足以解决问题：同一目标稍后还会用于背景覆盖优化、SaveLayer
合成以及最终画面提交，其中有直接解引用纹理的路径。

## 修复链路

修复位于引擎共享的 Impeller 层，没有应用业务层的平台判断。

1. `LazyRenderingConfig::Make` 统一检查目标、颜色纹理、深度和模板附件。
   构造函数私有化，根目标、离屏根目标、SaveLayer 和回切屏幕目标只能通过
   验证后入栈。根目标的深度或模板附件分配失败也立即拒绝该帧。
2. SaveLayer 失败使用现有的 `SkipUntilMatchingRestore` 平衡嵌套的 Save/
   Restore；失败目标不会覆盖有效的父目标。帧失败状态会保留到最终提交。
3. 整屏颜色覆盖优化复用已经验证的目标，只重建录制上下文，删除原先绕过
   统一创建入口的“弹栈再直接构造”路径；目标所有权收进配置内部。
4. `GetLocalCoverageLimit` 在没有有效目标时返回空覆盖范围。
5. `Canvas::EndReplay`、`CanvasDlDispatcher::FinishRecording` 和
   `RenderToTarget` 传递失败结果；`DisplayListToTexture` 对失败返回空纹理。
   画面录制失败时不执行最终屏幕拷贝或 onscreen 提交。Metal 的现有调用方在
   `RenderToTarget` 返回 false 时停止本次提交，后续帧可重新申请资源。
6. 最终拷贝和 mipmap 生成检查命令缓冲区、pass 创建及编码结果；失败的编码
   不在上下文析构时重试。

## 验证

原生回归用例使用真实后端，单独注入离屏渲染目标分配失败：

- `FailedRootReadbackTargetDoesNotAbortAndNextFrameRecovers`
- `FailedSaveLayerPreservesNestedSaveRestoreAndReportsFrameFailure`
- `InvalidRenderTargetReturnsFrameFailureRatherThanSuccess`
- `LazyRenderPassRejectsMissingAttachments`
- `FullCoverageDrawRestartsValidatedPass`

测试同时检查失败后缓存 Start/End 成对、嵌套恢复到正确栈深度，以及允许
分配后下一帧恢复；另有用例覆盖整屏绘制丢弃旧命令后继续绘制。

- macOS arm64 原生引擎测试目标已完整编译。
- 新增 5 项用例在真实 Metal 后端全部通过（952 ms）。
- 相关 SaveLayer、Backdrop、AdvancedBlend、Canvas 和渲染目标用例共 69 项；
  其中 68 项在默认模式通过，1 项需要原生绘制窗口，随后启用窗口独立运行通过。
- iOS arm64 模拟器引擎已完整编译，UUID 为
  `E2BA6AAA-67CC-3183-8B29-05527BFFCB0C`。
- App 已用隔离 SDK 的新引擎成功构建并启动，截图可见正常的初始页面；
  安装包内 Flutter 二进制 UUID 与新引擎一致，App 进程保持运行。
- 尚未验证 iOS 16 真机，也未构建本次修复的真机 Release 引擎。

本机为 Xcode 27 SDK，下载的 LLVM 21 链接器不认识新 SDK 的 `arm64e.x1`
TAPI 记录，隔离工具链改用 Xcode 链接器。编译驱动仅对旧编译器不认识的 SDK
属性保留警告，其他警告仍作为错误。没有修改系统 SDK、应用 SDK 或发布缓存。
定制引擎原有 iOS 平台配置缺少 `is_qnx = false`，在构建层补齐；Metal 格式
转换对该分支新增且未支持的 `kB10G10R10A2UNorm` 显式返回 Invalid；删除图片
解码器中已无调用的 `kShouldUseMallocDeviceBuffer` 常量。这三处属于原有源码
构建缺口，不改变应用业务逻辑。

本地引擎源码位于 `tools/release/work/impeller-engine-sdk` 的
`codex/impeller-render-target-failure` 分支，修复提交为
`9212bcc3d21`（本地提交，尚未推送）。构建记录位于
`tools/release/work/impeller-coverage-investigation`。

## 应用接入要求

修改 C++ 源码或热重载不会改变 App 内的 Flutter 引擎。必须使用同一基线重新
构建 iOS 引擎，验证模拟器与真机发布产物，再接入 SDK 的固定版本缓存来源。
在新产物接入之前，现有 `.operit.4` 仍然使用原 iOS 引擎，不能宣称 App 已修复。
本地验证仅将新模拟器引擎放入隔离 SDK 缓存；工程 `.fvmrc` 和正式 SDK 的
制品来源没有切换。尚未接入正式构建，也未发布 TestFlight 构建。
