# 移动端接收文件附件

Android 的“分享给 Operit2”和“使用 Operit2 打开”，以及 iOS 的文件打开入口，都将文件交给主窗口当前对话的附件上传流程。只添加附件，不发送消息。多文件操作按批次串行处理，保留文件名，使用现有附件保留大小限制和远端对话上传路径。

Android 注册 `ACTION_SEND`、`ACTION_SEND_MULTIPLE`、`ACTION_VIEW`，在发送方授予临时读取权限期间将内容复制到应用缓存，再通过 `operit/incoming_files` 的有界读取接口上传。`MainActivity` 同时处理启动 intent 和 `onNewIntent`。输入关闭时释放句柄并删除缓存副本。

iOS 注册 `public.data`、`public.content` 文档类型，并处理 Scene 的首次 URL 和运行中 URL。主应用和 `ShareExtension` 将文件复制到 App Group 收件箱，完成整批复制后原子发布。文件提供者的临时 URL 和 security scope 在复制完成之前保持有效；文件内容不通过 MethodChannel 整体传输。主应用在启动、恢复前台或收到扩展发布通知时读取新批次，成功或失败结束上传后清理对应输入。

分享扩展使用公开扩展 API，不主动拉起主应用。分享完成后提示用户打开 Operit2；应用回到前台后自动添加附件。App Group 内已发布的文件可以跨进程启动保留。主应用与其他应用并排显示时，扩展的发布通知也可直接唤醒主应用内部的接收队列。

## iOS 签名配置

- 主应用 Bundle ID：`app.operit`。
- 分享扩展 Bundle ID：`app.operit.ShareExtension`。
- 两者的 App Group：`group.app.operit.incoming`，由 Xcode 的 `OPERIT_INCOMING_APP_GROUP` 构建设置统一传入 Info.plist 和 entitlements。
- Apple Developer 中需要注册扩展 App ID，并为两个 App ID 启用同一个 App Group。现有主应用的手动分发 provisioning profile 需要包含这个权限；分享扩展也需要自己的 provisioning profile。Xcode 工程内的扩展使用 Automatic signing。
- `ShareExtension` 已作为 Runner 的依赖并嵌入 `PlugIns`，版本号取自 Flutter 生成配置。修改应用标识时，同时修改扩展标识和 App Group，并更新签名配置。

## 验证

在 `apps/flutter/app` 中执行：

```text
fvm flutter test --no-pub test/incoming_attachment_test.dart test/streamed_file_attachment_test.dart
fvm dart analyze lib/main.dart lib/core/host/IncomingAttachmentService.dart lib/ui/features/chat/screens/AIChatScreen.dart lib/ui/main/screens/OperitMainScreen.dart test/incoming_attachment_test.dart ../thirdparty/operit_folder_access/lib
```

自动化测试覆盖启动时排队、等待可用对话、批次串行执行、对话切换期间暂停投递、旧界面销毁不影响新接收者、Android/iOS 适配器的启动与运行中事件、实际附件上传接口、有界读取和输入释放。现有流式上传测试继续验证上传取消、大小限制和失败释放。

设备验收需要安装包含本次原生修改的新包：

1. Android 文件管理器选择一个 PDF，使用 Operit2 打开；检查当前对话出现待发送附件。分别在应用关闭和已运行时操作。
2. Android 多选图片和文档，分享给 Operit2；检查文件数量、文件名、内容以及附件预览。
3. iOS“文件”中打开文档并选择 Operit2，验证应用关闭和已运行两种情况。
4. iOS 分享面板选择 Operit2，等待扩展显示保存完成，再打开主应用；检查同一批文件只添加一次。另测主应用未启动时分享、重启主应用后接收，以及 iPad 并排显示。
5. 接收过程中切换对话，确认不会把旧对话的未完成上传附加到新对话；确认分享操作本身不发送消息。
6. 测试无权读取的提供者、超出附件保留限制的文件和多个同名文件，确认显示实际错误且不覆盖原文件。
