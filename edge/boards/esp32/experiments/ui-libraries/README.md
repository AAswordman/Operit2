# ESP32 UI 库对照试验

本目录保存可复现的候选适配、固定版本与测量记录。默认固件继续使用 `ui_port/operit_mini_ui.c`；这些试验不加入产品构建，也不刷写设备。

## µGUI 基础图元

基线为 `7605b2fd1c39b0c4a2c6db544615429a27fd7aa2` 的 remote-display 固件。µGUI 固定为 `sources.json` 中的提交，RGB565 配置关闭其内置字体。补丁使用真实的 `UG_FillFrame` 和 `UG_DrawLine`，像素回调仍写入原来的两行缓冲。原有字形、布局、交互和场景接口保持一致。这里只替换矩形和直线算法，没有迁移整个控件系统。

| 测量（2026-11-26） | 原实现 | µGUI |
|---|---:|---:|
| 真实应用镜像 | 2,889,056 B | 2,889,632 B |
| UI 相关目标静态符号之和，未计对齐 | 10,181 B | 10,189 B |
| `operit_ui_pump` 自身 Xtensa 栈帧 | 48 B | 160 B |
| 条带缓冲 | 1,280 B | 1,280 B |
| 桌面 100 帧 CPU 时间中位数，7 轮 | 0.088 s | 0.119 s |

两份真实固件均通过编译、ABI 和分区检查，原始检查结果在 `recorded/`。12 个完整画面逐像素一致，忽略实现相关的 `staticBytes` 后节点与状态 JSON 相同。桌面时间不是 ESP32 帧率，也不包含 SPI 传输；栈帧不是调用链栈峰值。初次使用静态 `UG_GUI` 超过 10 KiB 断言，因此最终候选将它放在 pump 的栈上，结束时清除库的上下文指针。

本候选比基线增加 576 B Flash，保留作为可复现对照。此结果不能代表其他库或完整控件迁移的效果。

## 复现桌面对照

需要 Python 3、Git、GCC；缺少上游源码缓存时还需要已配置的 GitHub CLI `gh`。Windows 使用 MinGW GCC。脚本按 `sources.json` 的提交和 SHA256 准备源文件，保留上游版权声明；已存在但校验不符的缓存会报错。准备好的缓存位于板卡 `.embuild/*-probe`。

在仓库根目录运行：

```powershell
$env:CC = 'D:/Program Files (x86)/mingw/mingw64/bin/gcc.exe'
python edge/boards/esp32/experiments/ui-libraries/compare-ugui.py
```

`CC` 可替换为本机 GCC 路径，默认使用 PATH 中的 `gcc`。输出目录为忽略的 `.embuild/ui-libraries/ugui-*`，保留两个可执行文件、12 组 RGB565/JSON、像素比较和交错运行 7 次的原始计时。补丁仅应用于隔离的源码副本。

桌面捕获使用公开 C API 覆盖初始聊天、远程首页、已配对、侧栏、配对码、空间审批、错误、远程场景、中文聊天和三种表情。它检查条带边界和每轮 12,000 次刷新。64 位 MinGW 的结构大小与目标不同，桌面构建关闭目标静态大小断言，并使用 `-mno-ms-bitfields` 匹配字形的打包格式。真实 Xtensa 编译保留静态断言。

## 复现 µGUI 完整固件

先运行上述脚本准备依赖。另用干净的实验分支或工作树，在仓库根目录应用两个补丁，再运行原来的完整检查入口：

```powershell
git apply edge/boards/esp32/experiments/ui-libraries/ugui-ui.patch
git apply edge/boards/esp32/experiments/ui-libraries/ugui-cmake.patch
node --experimental-strip-types edge/boards/esp32/editor/src/build.mts --firmware-check --remote-display
```

工作树需要自己的 `.embuild/ugui-probe` 缓存；在该工作树先运行桌面脚本即可准备。检查完成后可反向应用这两个补丁恢复默认实现。该流程只构建和检查，不执行刷写。最终实际 `.bin` 与固定分区比较才是 Flash 结论，不能按源码大小或 ELF 文件长度推算。

## 记录边界

`recorded/` 是历史测量快照，记录基线与候选的实际镜像 SHA256、分区检查、目标静态符号、逐像素比较和桌面计时。完整 ELF、bin、生成的画面与本机构建日志留在本地缓存，不提交到仓库。复跑会生成新记录，不会覆盖历史快照。

本轮库候选没有刷写 COM5；配对、Space 和任务猫的真机端到端行为不是这组桌面对照的验证范围。文件职责与构建输入见上一级 `build-audit.md`。

## U8g2 字体对照

```powershell
python edge/boards/esp32/experiments/ui-libraries/compare-u8g2.py
```

沿用上述 `CC` 设置。脚本按固定提交与校验和准备官方 `bdfconv` 和字体解码器，将当前字体完整导出为 BDF、编译官方转换器，再实际解码转换结果。生成的 BDF、字体 C、可执行文件和结果留在 `.embuild/ui-libraries/u8g2-*`。

转换使用 `-b 0 -f 1`，文字显式选择 `32-65535`，大数字选择 `48-57`。原渲染器对非零覆盖值使用同一前景色，BDF 按这个实际显示语义保留每个亮灭像素、偏移和 advance。校验器对每个字形调用 U8g2 的真实解码器，检查绘制像素和字宽。

| 字体数据 | 原格式 | U8g2 格式 |
|---|---:|---:|
| 文字位图与字形表 | 120,316 B | 132,249 B |
| 大数字位图与字形表 | 829 B | 328 B |
| 合计 | 121,145 B | 132,577 B |

2026-11-26 的记录位于 `recorded/u8g2-font-check.json`：4,168 个字形，0 个像素或字宽不匹配。数据总量增加 11,432 B，其中数字数据减少 501 B。文字与数字均保留为独立候选，以便后续测试局部采用及组合收益。

这里只比较字体数据，还没有加上目标解码机器码，也没有将 U8g2 接入完整固件。数字数据的减少不能直接称为固件减少 501 B，需加上解码器和适配后实际构建核对。生成器和桌面对照程序在电脑上运行，不属于 ESP32 的固件负担。
