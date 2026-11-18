# 工作区 VFS 挂载

## VFS 根目录

`/app` 和 `/operitapp` 同时挂到应用数据及工作区后端；`/mnt` 和 `/virtual` 同时挂到平台文件系统及授权资源后端。两组入口访问同一份数据。VFS 根入口通过 `hidden` 属性控制目录列表：`/app`、`/mnt`、`/sdcard` 和 `/data` 为隐藏入口，不出现在 `list /` 的结果中；直接指定路径仍可访问，列出其子目录也不受影响。根目录展示 `/operitapp`，存在可用平台挂载时再展示 `/virtual`。

VFS 直接解析这两组入口，不在插件、工具参数、返回值或持久化记录中将一个名字替换成另一个。文件结果保留调用者使用的 VFS 根目录。应用新生成的地址使用 `/operitapp` 和 `/virtual`，已有地址继续有效。该映射仅存在于应用内，不移动物理文件，也不修改终端中的实际挂载点。

## Host 文件选择结果

Compose DSL 文件选择器返回的 `path` 和 `uri` 都是同一个规范 VFS 路径，可以直接交给统一文件接口使用。原生物理路径、文档提供者 URI 和浏览器对象 URL 由 host 内部持有。

选中的文件通过 Flutter `XFile` 分块读取，经运行时的 `RuntimeStorageWriteHost` 写入私有暂存文件；字节数验证通过并提交后才返回 `/operitapp/data/temp/clean_on_exit/file_picker/<id>/<name>`。文件保留原始名称，属于退出时清理的临时数据，不参与同步。读取或存储失败会终止选择请求并清理本次产生的文件。`maxContentBytes` 只控制可选的内联 Base64 内容，未请求内联内容时同样返回可读取的 VFS 文件。

选中的目录保留平台适配器提供的授权后端，通过挂载目录注册为 `/virtual/<namespace>/<kind>/<id>`。Rust host 出口验证每个返回资源的 VFS 可访问性、类型和文件大小。

## Android 文档树

在聊天工作区中使用「选择本机文件夹」，Android 将打开系统 SAF 文档树选择器。选择 Termux 或其他文档提供者后，应用保留授权 URI，并将其注册为：

```text
/virtual/android/documents/<mount-id>/
/virtual/android/documents/<mount-id>/src/main.py
```

不把 `content://` URI 强行转换为物理路径，也不依赖提供者的 authority 或文档 ID 格式。挂载记录按当前运行时身份保存在 `config/vfs_mounts.json` 中；重启后仍需提供者存在且授权有效。授权撤销、只读树及提供者错误会作为文件操作错误返回。

支持列目录、元数据、文本/二进制读写、创建目录、删除、复制/移动、查找、内容搜索及 ZIP 压缩/解压。实际写入能力取决于提供者权限。移动通过复制后删除完成，不保证原子性。删除挂载根目录被禁止。

## Android 原生根目录

```text
/virtual/android/root/          -> /
/virtual/android/root/data/     -> /data/
```

这是路径映射，不是提权或内核挂载。访问仍受 Android 的 UID、SELinux 和已有权限限制。访问 Termux 私有目录应使用其授权的 SAF 文档树，不能靠 `/virtual/android/root` 绕过权限。

## 跨平台扩展

本机绝对路径的转换、内置挂载解析和目录展示共用 host 契约 `HostFileSystemRoots`，目标平台选择集中在该契约的分发模块中。各平台的原生根目录为：

| 平台 | VFS 根目录 | Host 位置 |
| --- | --- | --- |
| Windows | `/virtual/windows/<drive>` | 对应盘符根目录 |
| Android | `/virtual/android/root`、`/virtual/android/sdcard` | `/`、`/sdcard` |
| Linux | `/virtual/linux` | `/` |
| macOS | `/virtual/macos` | `/` |
| iOS | `/virtual/ios` | `/`，访问受应用沙箱及已有授权限制 |
| 鸿蒙 | `/virtual/ohos` | `/`，访问受应用沙箱及已有授权限制 |
| ESP32 | `/virtual/esp32` | 固件文件系统的 `/` |
| Web | 应用 VFS 根目录及已注册资源挂载 | 浏览器私有存储及授权资源 |

`/mnt` 对上述 `/virtual` 路径保持同等解析能力。鸿蒙的 Linux 内核不改变其 host 命名空间。Web 不接受本机 POSIX 目录；未声明文件系统的平台明确报错，不生成 Linux 路径。只有当前 host 声明的内置根目录才可访问。

设置中的原生存储位置与工作区绑定都按当前 host 转换。`/data/...` 在鸿蒙上属于其原生文件系统，`/sdcard/...` 在 ESP32 上属于固件文件系统；它们不会被当成 Android 的目录。插件的旧共享存储别名仍在独立的兼容入口处理。目录包含关系和文件变更通知直接比较已解析的后端位置，浏览器的相对私有存储位置也不再进入原生路径转换。

通用挂载目录记录 `namespace`、`backend`、`root`、`name` 和稳定 ID。原生目录使用 `native` 后端；平台文档资源使用专属后端。新增非原生后端还需要实现对应宿主文件操作，注册名称本身不会产生访问能力。

命令接口：

```text
storage mounts list
storage mounts add /virtual/local/folders native /absolute/project Project
storage mounts remove /virtual/local/folders/<mount-id>
```

桌面工作区文件夹选择也通过该注册接口建立原生目录挂载，保留既有 macOS 安全作用域选择流程。Android 启动/设置中的应用存储根目录仍要求原生路径，不使用 SAF 工作区选择器。

## 限制与验证

工作区绑定的目录保存为 `nodeId://virtual/...`，其中节点是目录的所属设备，与聊天执行设备无关。文件面板、预览、保存、复制移动和 @ 引用通过现有文件工具定位所属节点，再访问该节点自己的 VFS 与挂载登记表。本机地址仍由本机处理；设备不可达时直接报告错误。文件工具同时接受原有的 `nodeId:/virtual/...` 地址。

已有工作区若只保存了裸挂载路径，跨设备访问前需要在目录所属设备重新绑定，以保存明确归属；不根据聊天执行设备推测旧目录的所有者。

SAF 是应用级文件访问能力，不是终端的物理工作目录；不能直接作为 shell cwd。终端、OCR 等仅支持原生文件路径的调用会明确拒绝文档资源定位符。

已有 Rust、Flutter 及 Kotlin 本地回归测试。完整 APK 构建、真机 Termux 提供者访问和跨重启授权恢复仍需设备验证。
