# 主动消息投递诊断

预约、聊天写入、界面状态和系统通知分别判断。`delivered` 仅确认 SQLite 消息已保存；没有用户可见性确认。通知 Host 接受不等于系统展示，失败记录在 `notificationError`，不改变已完成的聊天写入。

排查使用同一 `chatId` 与消息 `timestamp` 串联 `runtime/scheduled_chat_messages.json`、数据库和日志：

1. 核对数据库中的消息、正文、显示模式及角色扩展；预约状态不能替代数据库核验。
2. Core 的 `ChatFlowTrace publish` / `publish.done` 记录消息流发布。主动消息的即时提交、到期提交、入库后恢复均通过运行槽共享入口发布；去重恢复读取原数据库记录，不改写作者、生成时间或用量。
3. Flutter 的 `ChatFlowTrace chat.messages_applied` 记录聊天 ID、列表数量和首尾时间戳。它确认 UI 状态接收，不确认布局或视口可见。若流中存在而现场看不到，还需检查展示插件和滚动位置。
4. 通知字段分别表示尝试、Host 接受和错误。旧 Android 记录中的两个 `false` 没有未发送的证明力。通知事件通过现有 Host interaction 流传递，等待前先注册唤醒；同一客户端并行收到相同待处理事件时合并处理和 ack。

主动消息恢复在应用初始化中启动，等待插件目录加载完成后才核对角色上下文，避免构造运行时期间使用尚未预约加载的目录。目录失败保留原始错误，不以默认角色继续恢复任务。

`Scheduled reply preparation was interrupted` 表示持久化的 preparing 记录已没有生成任务；`Target conversation is processing` 表示准备时目标忙；上下文或参与者改变会撤销旧正文。多条提前准备的消息中，一条真实写入也会改变其余正文依据的历史，因此其余预约可能被判定上下文变化。

Flutter 框架错误保存完整 `FlutterErrorDetails`，包含 library、context 与 image provider 的诊断。共享 VFS 图片加载器额外记录资源路径、字节数、推断 MIME、原始错误与栈，即使控件使用 errorBuilder 也保留诊断。解码失败时无法承诺取得尺寸；当前没有为全部图片调用点增加消息 ID，网络及 inline 资源依赖其 provider 的框架诊断。

本次源码检查未取得反馈现场的任务文件、数据库或日志，不能将两条消息唯一归因于某个修复，也未将 ANR、网络中断及已知语音通话版本问题合并解释。
