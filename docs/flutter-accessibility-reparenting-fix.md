# Flutter AXTree 子树重挂载修复

适用基线：`AAswordman/flutter-ohos`，提交 `bc960fc88f6`，
`3.41.10-ohos-0.0.2-beta.operit.2`。

## 问题与修复

`AccessibilityBridge::CommitUpdates` 分两阶段移动节点：从旧父节点移除，
再挂到新父节点。AXTree 的移除操作会删除整棵子树，而 Flutter 的语义更新
只包含变化的节点。原实现假设被移动节点及所有后代都会出现在当帧更新中，
未保存未变化的节点；排序也只沿当帧更新的节点遍历。

例如旧树 `123 → [1 → 3 → 452, 2]`，将 `1` 移到 `2` 下，同时更新
`452` 的文本，`3` 保持不变。第一阶段删除 `1、3、452`；第二阶段缺少
`3` 的数据，无法建立到 `452` 的父子关系，对应
`452 will not be in the tree and is not the new root`。
该结构问题与日志相符，实际应用触发还需使用新二进制确认。

修复在移除前保留旧子树的 AXNodeData，将当帧更新合入这些数据；随后沿
最终树结构按父节点先于子节点提交。遍历经过未变化的中间节点，只提交
变化或因重挂载而需要重建的节点，不重新加入当帧已经移除的后代。
祖先和后代同时移动时，第一阶段只更新仍然存在的旧父节点。
焦点和选择信息也只从最终树中实际提交的语义节点更新。

排序遍历的成本为最终树节点数的线性量级；旧子树数据仅在重挂载时保存，
且同一批次每个旧节点只保存一次。实现位于共享桥接层，无新增平台分支。

Windows 的 `tooltip is not supported` 是另一条消息通道警告，
不属于 AXTree 更新错误。本补丁未改变该消息通道。

## 发布与验证

已发布 [`3.41.10-ohos-0.0.2-beta.operit.3`](https://github.com/AAswordman/flutter-ohos/releases/tag/3.41.10-ohos-0.0.2-beta.operit.3)，
维护仓库提交为 `f0c8e62c14b495e9801f7459366b0da4d61cebdf`。
项目的 `apps/flutter/app/.fvmrc` 已切换到 `.operit.3`。
已通过 FVM 安装并执行 `fvm flutter precache --windows`；缓存版本为 `.operit.3`，
下载后的三种模式 DLL 和 `gen_snapshot` 的 SHA-256 均与发布产物一致。

- Windows x64 Debug、Profile、Release 引擎均已编译、打包发布。
- 共享 C++ 测试 234 项通过，包括全部 17 项桥接测试及 3 项新增回归测试。
- Windows 无障碍与原生组合相关测试 43 项通过。
- Flutter 工具缓存和桌面编译器测试 83 项通过，2 项上游既有测试跳过。
- Profile/Release 新构建的 AOT 编译器共 4 项 ABI 检查通过。
- 修改的 Dart 源码通过分析和格式检查。
- 三个引擎 ZIP 的 CRC、每个文件的 SHA-256、源码版本及构建产物一致性均已验证。
- GitHub 的全部六个附件大小及 SHA-256 与本地一致；发布附带测试结果和校验清单。

引擎 ABI pin 保持 `42d3d75a56efe1a2e9902f52dc8006099c45d937`，
使用匹配的独立桌面编译器。Web 继续使用 `.operit.2` 的已验证产物，
此次仅发布 Windows x64 原生引擎，未重建 OHOS 等其他平台。
尚未重放原应用中的 GUI 触发流程。

新增用例为 `AccessibilityBridgeTest.ReparentAcrossUnchangedIntermediateNode`、
`AccessibilityBridgeTest.ReparentSubtreeWhileRemovingDescendant`、
`AccessibilityBridgeTest.ReparentAncestorAndDescendantTogether`，位于
`engine/src/flutter/shell/platform/common/accessibility_bridge_unittests.cc`。
它们属于 `common_cpp_unittests`，不是 `flutter_windows_unittests`。

需要单独重跑回归测试时，在引擎输出目录执行：

```powershell
./common_cpp_unittests.exe --gtest_filter=AccessibilityBridgeTest.*
```

应用侧要使用重新构建的共享桥接层所在引擎二进制；仅修改 SDK 中的
C++ 源码或执行热重载不会使本修复生效。
