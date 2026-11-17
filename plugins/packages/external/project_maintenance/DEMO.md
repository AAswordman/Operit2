# 本地维护演示

这个实验演示如何将一个独立 Git 仓库绑定到 Operit 聊天，让聊天中的反馈形成工单并由后台代理处理。先在运行节点准备好 Git、Node.js、可用的模型配置，以及一个**干净、有初始提交的专用测试仓库**。运行节点必须持续在线；当前实验不提供离线设备唤醒，也不是系统级仓库隔离。

## 安装插件

在 Operit2 仓库根目录构建插件（需要先安装仓库已有的 TypeScript 构建依赖）：

```sh
node plugins/packages/external/subagents/scripts/pack.mjs
node plugins/packages/external/project_maintenance/scripts/build.mjs
```

使用你希望承载工单的 Operit 身份和空间，导入并启用子代理与项目维护插件。以下命令使用 CLI 当前配置的身份；如果 GUI 使用不同身份或空间，工单不会自动出现在那个 GUI 中。

```sh
apps/cli/target/debug/operit2 cli plugin import plugins/packages/external/subagents/dist/subagents.toolpkg
apps/cli/target/debug/operit2 cli plugin enable com.operit.subagents
apps/cli/target/debug/operit2 cli plugin import plugins/packages/external/project_maintenance/dist/project_maintenance.toolpkg
apps/cli/target/debug/operit2 cli plugin enable com.operit.project_maintenance
```

如子代理属于应用内置包，按宿主提示从插件列表加载，而不是覆盖内置包。插件是否可用可用 `cli plugin list` 检查；侧边栏入口叫“维护中控”。首次使用还需要为父聊天配置可用模型、工具权限和运行节点。不要把 API 密钥放进命令、仓库或工单。

## 绑定并提交

1. 创建或选择专门用于反馈的聊天。先使用软件自带的工作区功能，将专用 Git 仓库绑定到该聊天；再在“维护中控 → 渠道连接”选择该聊天并启用收集，无需重复填写仓库路径。
2. 在聊天里描述问题，可以分成几条消息。例如：“分数好像不对”“我点了一次却加了两分”“应该每次只加一分”。消息持久化后，系统通常等待约 12 秒的安静期再聚合并启动诊断；持续聊天会在约 60 秒触发一次聚合。
3. 打开“维护中控”查看工单、诊断和验收记录；也可以在原聊天中询问进度。想直接提交一条完整反馈，可在面板选择“提交反馈”。
4. 查看测试仓库的 `git status` 和 `git log`，以及工单中的独立 Git 核验结果。没有配置远端时，成功交付只会创建本地提交，不会有 PR。

同一个仓库的工单会排队。信息不足的工单可在详情页补充；状态不明或中断的工单必须先检查代理、工作区和远端结果，不能假定其安全结束。这个演示不会自动合并代码。

QQ、微信接入需要安装对应 external Bot，配置账号，并从维护中控自动发现的 Bot 会话中开启维护，复用对应软件聊天的工作区；具体配置与已知的通知、附件和恢复限制见同目录的 README。真实账号、移动设备和真实远端 PR 需要单独验证。
