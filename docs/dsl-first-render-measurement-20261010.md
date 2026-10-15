# DSL 首次构建分段测量（2026-10-10）

## 范围

本机 AMD Ryzen 9 7940HX，Windows。没有运行中的 Operit；现有可执行文件为 2026-10-07，未启动它测当前源码。没有编译应用或读取实际用户数据。

本报告不是 QuickJS → CoreLink → Flutter 的实机端到端数据。两段分别运行，不能相加后作为点击到显示的耗时。

- JS：Node v24.11.0/V8，当前 selector TypeScript 打包为 CommonJS 后读取，生产 Acorn/DSL 编译器及 SDK，独立 VM 上下文，合成已加载的 20/100/500 行。开发期 TypeScript 打包不计时。包含脚本文件读取、SDK 初始化、完整 selector 模块编译执行、首次节点生成和 retained commit。文件系统为热缓存；不含业务读取、原生 Host 启动、JS→Rust 转换和 CoreLink 编码。每组预热后 15 次。
- Flutter：FVM debug widget test，直接使用生产 retained node store 和 renderer。离线 JSON 测试夹具解析在计时外；实际生产不走这里的 JSON。计入节点提交以及 pumpWidget 后的 Dialog 路由帧，断言第一行已经存在。包含测试环境启动 MaterialApp 的成本，不含真实 GPU 提交/上屏或弹窗动画完成。每组 3 次预热后 15 次。

## 最后一轮结果（ms）

| 行数 | 节点数 | JS 总 p50 | JS 总 p95 | Flutter 提交+测试帧 p50 | Flutter p95 |
|---|---|---|---|---|---|
| 20 | 189 | 31.87 | 54.47 | 36.53 | 44.46 |
| 100 | 909 | 56.99 | 107.83 | 29.27 | 37.58 |
| 500 | 4509 | 173.46 | 373.85 | 30.73 | 35.97 |

500 行的 JS 首次 render+commit p50 为 142.75 ms，其中 retained commit 分项 p50 为 90.51 ms；分项分位数不能直接相加。Flutter 500 行的节点提交 p50 为 3.31 ms，可见内容的测试帧约 27.84 ms。首个 Flutter 冷样本为 432.29 ms，单样本包含测试环境/JIT/首次控件初始化，不代表产品冷启动。

本次确认的是首次完整节点生成/commit 随列表规模增长。dirty-scope 改善后续变更，不会免除首次注册全部节点的成本。尚未测业务数据查询和完整运行时链路；不能声称已找到产品端到端最大瓶颈。跨轮波动明显，15 次的 p95 接近样本最大值。

## 复现

仓库根目录：

```powershell
node tools/performance/compose_dsl_first_render_probe.mjs
```

`apps/flutter/app`：

```powershell
$env:RUSTFLAGS='-Awarnings'
fvm flutter test --no-pub --no-test-assets --dart-define=OPERIT_DSL_FIRST_FRAME_PROBE=true test/compose_dsl_first_frame_probe_test.dart --reporter expanded
```

Flutter 测试通过。该探针默认跳过，避免普通回归依赖本地生成的夹具。报告及源码 SHA256 在 `target/dsl-performance/first-render-*.json`。新增 renderer 入口仅用于测试，未改生产渲染逻辑。
