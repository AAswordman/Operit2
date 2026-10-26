# 工作区 VFS 挂载

## Host 文件选择结果

Compose DSL 文件选择器返回的 `path` 和 `uri` 都是同一个规范 VFS 路径，可以直接交给统一文件接口使用。原生物理路径、文档提供者 URI 和浏览器对象 URL 由 host 内部持有。

选中的文件通过 Flutter `XFile` 分块读取，经运行时的 `RuntimeStorageWriteHost` 写入私有暂存文件；字节数验证通过并提交后才返回 `/app/data/temp/clean_on_exit/file_picker/<id>/<name>`。文件保留原始名称，属于退出时清理的临时数据，不参与同步。读取或存储失败会终止选择请求并清理本次产生的文件。`maxContentBytes` 只控制可选的内联 Base64 内容，未请求内联内容时同样返回可读取的 VFS 文件。

选中的目录保留平台适配器提供的授权后端，通过挂载目录注册为 `/mnt/<namespace>/<kind>/<id>`。Rust host 出口验证每个返回资源的 VFS 可访问性、类型和文件大小。

## Android 文档树

在聊天工作区中使用「选择本机文件夹」，Android 将打开系统 SAF 文档树选择器。选择 Termux 或其他文档提供者后，应用保留授权 URI，并将其注册为：

```text
/mnt/android/documents/<mount-id>/
/mnt/android/documents/<mount-id>/src/main.py
```

不把 `content://` URI 强行转换为物理路径，也不依赖提供者的 authority 或文档 ID 格式。挂载记录按当前运行时身份保存在 `config/vfs_mounts.json` 中；重启后仍需提供者存在且授权有效。授权撤销、只读树及提供者错误会作为文件操作错误返回。

支持列目录、元数据、文本/二进制读写、创建目录、删除、复制/移动、查找、内容搜索及 ZIP 压缩/解压。实际写入能力取决于提供者权限。移动通过复制后删除完成，不保证原子性。删除挂载根目录被禁止。

## Android 原生根目录

```text
/mnt/android/root/          -> /
/mnt/android/root/data/     -> /data/
```

这是路径映射，不是提权或内核挂载。访问仍受 Android 的 UID、SELinux 和已有权限限制。访问 Termux 私有目录应使用其授权的 SAF 文档树，不能靠 `/mnt/android/root` 绕过权限。

## 跨平台扩展

通用挂载目录记录 `namespace`、`backend`、`root`、`name` 和稳定 ID。原生目录使用 `native` 后端；平台文档资源使用专属后端。新增非原生后端还需要实现对应宿主文件操作，注册名称本身不会产生访问能力。

命令接口：

```text
storage mounts list
storage mounts add /mnt/local/folders native /absolute/project Project
storage mounts remove /mnt/local/folders/<mount-id>
```

桌面工作区文件夹选择也通过该注册接口建立原生目录挂载，保留既有 macOS 安全作用域选择流程。Android 启动/设置中的应用存储根目录仍要求原生路径，不使用 SAF 工作区选择器。

## 限制与验证

SAF 是应用级文件访问能力，不是终端的物理工作目录；不能直接作为 shell cwd。终端、OCR 等仅支持原生文件路径的调用会明确拒绝文档资源定位符。

已有 Rust、Flutter 及 Kotlin 本地回归测试。完整 APK 构建、真机 Termux 提供者访问和跨重启授权恢复仍需设备验证。
