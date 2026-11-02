# AArch64 架构审计

本目录把目标范围、独立测试输入和实际结果分开管理：

- `architecture.json`：架构基线、功能域、已知缺口和 Linux 环境风险。`complete` 当前为 `false`。
- `catalog.py`：手工编写的指令形式清单，按指令族生成宽度、向量形状和立即数组合。不读取解释器的解码表或实现代码。
- `harness.c`：固定边界值、可复现随机输入、16 种 NZCV、六种 FPCR；向同一汇编样本提供数据。
- `audit.py`：将样本分别编译为宿主 ARM64 程序和 Linux AArch64 ELF，每个形式独立执行，比较通用/向量寄存器、NZCV、FPSR 和 256 字节访存区域。

运行完整的现有样本集并检查寄存器重叠：

```sh
python3 hosts/ios/linux-runtime/tests/isa/audit.py \
  --runtime tools/ios-runtime/arm64/build/host/operit-arm64 \
  --report tools/ios-runtime/arm64/build/isa-audit.json \
  --register-aliases --require-passing
```

可以多次使用 `--family` 选择一个或多个域。`--require-passing` 让未支持、执行失败和结果不一致均导致非零退出。完整报告放在构建目录，持续集成在原生 ARM64 宿主上运行此检查；其他宿主不能伪造硬件对照结果。

`passed` 仅证明该形式、这些输入和所观察状态的对比通过，不能据此宣称该指令族或整个 ISA 已完成。未进入清单的已知形式仍记录在 `architecture.json` 中。新增指令族时应先补入清单和保留编码检查，再完成实现；Node/Python 探针只用于应用验收。

普通三操作数向量的重叠测试覆盖 Rd=Rn、Rd=Rm、Rn=Rm；当前尚未穷举所有 32 个寄存器、所有立即数和所有编码约束。FMOV 立即数例外：已枚举全部 256 个编码值及标量/向量的五种形式。

浮点宿主指令均在构建时静态汇编。执行前后保存恢复宿主 FPCR/FPSR，guest 字节从不作为宿主可执行代码。没有引入外部模拟器实现。
