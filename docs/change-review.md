# 变更审阅的工作区路径

变更审阅插件位于 `plugins/packages/buildin/change_review`。工作区输入可以是当前
Core 的 VFS 地址，也可以是绑定设备的地址，例如
`core-linux://virtual/linux/home/harmoon/projects/Operit2`。

文件列表和内容读取使用已有文件工具的节点路由。`workspace exec`、
`workspace resolve-path` 和 `workspace open-path` 是目标 Core 内的命令，
只接受该 Core 的本地路径。插件在命令边界拆分设备标识和路径，通过
`Tools.device(nodeId).SoftwareSettings.exec` 把命令发给所属设备，再向目标
Core 传递 `/virtual/...` 地址。没有设备标识的路径使用当前 Core。

Git 返回的仓库根目录是执行设备的原生路径。读取未跟踪文件、完整文件内容和
打开文件时，插件使用该仓库所属工作区选择执行设备，在目标 Core 上通过
`resolve-path --native` 转换，然后把设备标识附回返回的 VFS 地址。
来自上一轮变更的相对文件路径同样以所属工作区为基准解析；显式指定其他设备
的文件地址在指定设备上解析，不使用当前工作区的目录。Windows 盘符与 VFS
文件名中的冒号不表示设备前缀。

设备不可达或 Git 查询失败时，错误会直接显示为“无法加载文件变更”，保留
具体错误内容。不会改在当前设备执行，也不会把失败显示成正常的空变更列表。

检查命令（在插件目录执行）：

```text
pnpm run check
node --test tests/*.test.mjs
```

回归检查覆盖带设备前缀的 Linux 工作区、Git 命令派发、未跟踪文件读取、
原生根目录转换、外部打开、相对资源解析、Windows 盘符以及设备不可达。
真实设备的跨节点连接和系统打开行为仍需设备验证。
