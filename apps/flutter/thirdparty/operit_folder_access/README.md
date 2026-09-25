# operit_folder_access

主应用目录选择的统一入口（单选和多选）：

```dart
final folder = await OperitFolderAccess.pickDirectory();
final folders = await OperitFolderAccess.pickDirectories();
```

两者均支持 `initialDirectory`。取消分别返回 `null`、空列表；权限或平台不支持错误向调用方传播，不伪装成取消。

- macOS：Flutter 插件注册选择本包的原生实现，NSOpenPanel 选择目录，并由原生进程级 FolderAccessStore 保存和恢复 security-scoped bookmarks。多选逐一处理授权；任何一项失败，不返回部分成功的路径列表（之前成功保存的授权仍然保留）。
- 其他平台：本包内部委托 file_selector，能力取决于对应平台实现；统一 API 不代表每个平台都支持目录多选。
- 主应用不判断操作系统或发行渠道，也不直接调用 file_selector 的目录接口。普通文件打开、保存不属于本包。
- 返回值保留底层路径/URI，不进行 VFS 路径映射，也不代表远程主机拥有该路径或权限。

当前面向启用 App Sandbox 的 macOS 应用。签名与授权策略在原生层，不要求主应用在将来增加非沙盒发行版时添加平台分支。重复插件注册不重复开启已恢复的 scope；书签指向已移动目录时，不将旧配置路径视为已授权，需要重新选择。

macOS 宿主需保留 App Sandbox，并启用 user-selected read/write 与 app-scope bookmarks entitlement。实际签名沙盒下的授权恢复需要真机测试，Dart 通道测试不能替代它。
