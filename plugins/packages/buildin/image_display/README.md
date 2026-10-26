# 图片展示

空间安装的轻量 ToolPkg：通过系统提示词告诉 AI 图片保存位置和标准 Markdown 写法，复用现有聊天图片渲染链路。

## AI 输出

先通过已有文件工具将真实图片复制、下载或写入 `ToolPkg.getSpaceDataDir()` 返回的目录，确认操作成功后输出：

```markdown
![图片说明](/app/data/plugin_data/space/com.operit.image_display/image-unique.png)
```

图片语法独占一行，直接写入回复正文。每张图片使用唯一文件名，并保留已被历史消息引用的文件。

## 架构

- `installation_scope: "space"`：插件安装在共享空间。
- `ToolPkg.getSpaceDataDir()`：由 Core 创建并返回插件共享数据目录，提示词在运行时获取该路径。
- `after_compose_system_prompt`：追加中英文图片规范，保留已有系统提示词。
- 标准 Markdown 图片组件通过现有 Core VFS 二进制文件服务读取图片，再由 Flutter 解码显示。
- 插件不注册业务工具或自定义渲染器，文件操作使用现有工具。

共享目录属于现有空间同步范围；图片在接收端可用的时间由空间文件同步决定。图片规范只控制回复展示，不会把图片自动作为模型的视觉输入。
