---
name: "PackageBuilder"
description: "使用当前 Operit 随软件携带的类型定义和示例开发、打包、安装与测试插件包；通过现有 host API 核对环境与路径。"
---

# PackageBuilder

这是当前 Operit 随软件携带的插件开发 Skill。类型和示例来自当前版本内置资源，不从云端拉取其他版本。

## API 版本与跨平台要求

Operit2 的 ToolPkg API 支持版本为 `2.0.0`；对 `1.0.0` 和 `1.0.1` 的加载支持并不完整。Operit1 完整支持 ToolPkg API `1.0.0` 和 `1.0.1`，这两个版本主要面向 Android，是旧版 API 形式。

为 Operit2 新建 ToolPkg 显式声明 `"api_version": "2.0.0"`，使用本 Skill 的当前版本类型。不能把旧版兼容加载当作完整支持，也不能只修改旧包版本号就宣称完成迁移。

ToolPkg API `2.0.0` 面向多平台开发，基本所有公共接口都通过统一 host 能力提供跨平台兼容。常规功能直接使用当前版本接口即可，不必为每个平台重复实现。作者仍需注重跨平台兼容性：遇到平台特异接口时，明确其适用范围，考虑并验证其他平台的安装、界面与核心功能；不能把某个平台的专用能力当作所有平台都具备的接口。

## 随包资料

```text
PackageBuilder/
  SKILL.md
  references/
    PLUGIN_CREATION_WORKFLOW.md
    SCRIPT_DEV_GUIDE.md
    TOOLPKG_FORMAT_GUIDE.md
  types/
    index.d.ts
    core.d.ts
    toolpkg.d.ts
    ...
  examples/
    packages/
      buildin/
      external/
```

示例目录提供开发源码与构建配置，不携带 `dist/`、`build/`、`tests-dist/` 中的生成文件或成品包。Compose UI 按普通、可读的 CommonJS JavaScript 交付，保留界面声明和回调，运行时负责 Compose 编译与缓存。不要指导用户生成或修改 Compose 内部预编译产物。

## 创作步骤

1. 阅读 `references/PLUGIN_CREATION_WORKFLOW.md`。通过 operit_editor 工具的 `args: ["skill", "show", "PackageBuilder"]`（或系统工具 `execute_cli_command`） 读取真实 Skill 目录，通过终端 host 信息和实际文件访问确定用户同意使用的 `<dev_root>`；不要写死平台目录，也不要把 VFS 地址直接当作终端或导入路径。
2. 确定稳定的 package id，在 `<dev_root>/<package_id>/` 保存项目，把本 Skill 的 `types/` 完整复制到 `<dev_root>/types/`。继续开发时先核验原项目与源码，不根据已安装包名声称源码已找到。
3. 使用当前类型定义和 `examples/packages/buildin/`、`examples/packages/external/` 中实际存在的示例。脚本开发看 `references/SCRIPT_DEV_GUIDE.md`，ToolPkg 清单、资源和 UI 注册看 `references/TOOLPKG_FORMAT_GUIDE.md`。
4. 使用 TypeScript 编写源码并输出 CommonJS JavaScript，保留 `.ts`、`tsconfig.json` 和最终产物。项目按 `../types` 引用声明；在 `src/` 内直接引用声明时使用 `../../types/index.d.ts`。
5. 通过统一文件、网络与终端 host API 开发和打包，确认工具与路径可用。ToolPkg 归档根目录包含 manifest，成品放在源码目录之外，并包含二次开发源码。
6. 调用 operit_editor 包的 `operit_editor` 工具，通过 `args` 字符串数组实际执行 `package import/enable/list/show/exec` 与 `log package`；`args: []` 读取命令帮助。该工具接入 `Tools.SoftwareSettings.exec(args)`，原样返回实际执行结果与错误，不接受自然语言 `query` 或任意脚本片段。变更前取得用户确认，逐项核验导入结果、真实启用状态及工具或界面测试结果。
7. 同 ID 更新采用明确授权的删除、重新导入、按记录恢复启用状态和重新测试流程；`package import` 不覆盖已注册包。遇到错误停止并定位，不改 id、不宣称成功。
8. 交付源码、成品、安装与测试结果。市场发布是用户确认后的独立操作，不能把本地测试等同于发布成功。

包管理创作入口只准备 Skill 和包，并把需求填入聊天草稿；用户发送后才开始开发。它不自动生成、构建、安装或发布插件。
