---
name: "PackageBuilder"
description: "使用当前 Operit 版本随软件携带的类型定义、教程和示例开发 Operit 插件包。"
---

# PackageBuilder

这是当前 Operit 版本随软件携带的插件包开发 Skill。结构沿用 Operit1 的开发 Skill：教程在 `references/`，类型在 `types/`，示例包在 `examples/packages/`。当前版本不从云端拉取内容；`types/` 和示例包由软件内置资源提供。

## 目录

安装后的 Skill 目录应具备这组结构：

```text
PackageBuilder/
  SKILL.md
  references/
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

开发目录固定为：

```text
/sdcard/Download/Operit/dev_package/
  types/
  <package_id>/
    manifest.json
    tsconfig.json
    src/
    dist/
```

`types/` 是各个包项目的兄弟目录。包项目内部通过 `../types` 引用类型定义。

## 工作要求

- 新建插件包时先确定稳定的 package id，后续保持不变。
- 开始开发前把本 Skill 的 `types/` 复制到 `/sdcard/Download/Operit/dev_package/types/`。
- 在 `/sdcard/Download/Operit/dev_package/<package_id>/` 中开发、安装和测试。
- 使用 TypeScript 编写源码，保留 `.ts` 源码、`tsconfig.json` 和最终 `dist/` 产物。
- `tsconfig.json` 的 `typeRoots` 和 `include` 按 `../types` 组织。
- ToolPkg 以 `manifest.json` 或 `manifest.hjson` 描述包元数据、资源、界面入口与子包。
- `references/SCRIPT_DEV_GUIDE.md` 和 `references/TOOLPKG_FORMAT_GUIDE.md` 作为教程资料；实际接口以 `types/*.d.ts` 为准。
- `examples/packages/buildin/` 和 `examples/packages/external/` 作为当前版本包结构示例。
- 安装与测试使用当前 runtime 的 package core command；需要平台编辑能力时读取 `operit_editor` 包说明后调用 `execute_cli_command`。
