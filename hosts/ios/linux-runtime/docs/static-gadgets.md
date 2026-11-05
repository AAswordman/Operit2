# 静态 ARM64 gadget 解释器

`src/cpu/aarch64/gadgets.S` 提供构建时编译的 threaded interpreter。解码阶段只创建
`NativeGadget` 数据记录，其中的处理器地址指向已链接的汇编代码；运行时不分配可执行内存，
不生成、复制或修改宿主指令。它和摘要固定的 musl 函数替换是两条独立路径。

## 执行与回退

`Interpreter::decode_block` 为缓存块建立连续、32 字节一条的 gadget 记录。至少两个相邻
指令有处理器时，C++ 分派可进入汇编；每个处理器通过自己的间接跳转位置分派下一条，避免
编译器把所有分派尾部合并。寄存器值仍保存在 `CpuState`，guest NZCV 在一次汇编调用内保留
在宿主寄存器中，退出时写回。

操作记录的处理器地址已细分为确定的 32/64 位加减、逻辑、进位和乘法变体；执行阶段无需
再次判断这些运算的立即数/寄存器形式、加减方向或是否更新标志。`gadget_entries.inc`
同时定义 C++ 索引、汇编实例与地址表，避免三处列表漂移。寄存器字段是 `CpuState` 槽位，
SP 与 ZR 在译码时解析。ZR 映射到新增的只读零值源槽（每个 CPU 状态增加 8 字节），每次
进入汇编时清零；丢弃的目的寄存器仍跳过写回，不能污染后续 ZR 读取。

汇编是叶函数，执行上下文使用 x0–x7，处理器临时值使用 x8–x17。它不建立栈帧，不读写
callee-saved 寄存器，也不使用 Apple 保留的 x18。常用 64 位单值/成对固定偏移访存另有
确定地址模式的处理器，省去通用索引扩展、后索引选择与回写判断；TLB、页内范围、权限、
共享存储与文件代数检查相同。成对加载按第一目的、第二目的的原顺序回写，保留别名语义。

这些仍是逐条分派的解释器优化；没有块级原生代码编译器、跨指令寄存器分配或整个热点块
的原生执行。不能把局部提速当作任意新程序可获得 JIT 性能的证明。

目前覆盖 32/64 位立即数装载、保留半字、ADR/ADRP、无移位的加减/逻辑运算、ADC/SBC、
普通乘加/乘减、条件选择、寄存器搬运、标量单值/成对读写。B.cond、CBZ/CBNZ、TBZ/TBNZ
仅在分支未跳转时连续执行；需要跳转时先返回 C++。浮点/SIMD、移位/扩展运算和其他未覆盖
指令使用已有处理器。汇编不修改 FPCR/FPSR，不使用 Apple 保留的 x18。

返回值是已完成的指令数，PC 与退休计数由 C++ 更新。预算耗尽、未知处理器、分支跳转或
内存检查不通过都会停止；当前指令保留给 C++，不会提前执行寄存器回写。原始 opcode 和
块 epoch 仍保留，原有代码失效、重解码、块缓存轮换与 syscall 调度继续生效。

## 内存边界

`Memory::gadget_memory` 只在地址空间处于串行执行模式时提供当前 TLB 和 epoch。汇编对每次
访问检查 48 位地址范围、4 KiB 页内范围、TLB tag/epoch 和权限；未驻留、TLB 未命中、
跨页、共享映射、文件底层存储、可执行目标写入都回到 C++。私有文件页完成 COW 后，
只有当前 ExecutionScope 已验证文件大小、且 TLB 的 file_generation 与该 scope 一致时，
才允许直接访问其私有 RAM。新 scope 或 scope 外必须先回退检查，文件截断仍产生 Bus 故障。
成对访问检查完整 8/16 字节范围后才读写，不执行半个访问。

fork COW 页的 TLB 不带写权限，因此必须先由普通内存路径完成复制。成功的原生写入清除
页独占保留 token，使后续独占写失败；失败的检查不改数据、保留 token 或基址。并行执行
整体使用原解释路径。汇编不更改映射、申请页、调用宿主函数或抛出 C++ 异常。

`gadget_offsets.h` 和 `memory/translation_layout.h` 是 C++/汇编共享 ABI。字段偏移和结构
大小有编译期断言。修改寄存器或 TLB 布局时须同步这些常量及测试。每条缓存指令目前新增
一条 32 字节 gadget 记录和一个指针，因此会增加解码缓存内存；这不是整个 runtime 的内存
占用数字。

## 验证与测量

`arm64-gadgets` 把汇编执行与原 `execute` 做寄存器/条件码差分，并检查单值及成对内存、
分支、预算切分、缺页/保护错误的退休位置、并行回退、跨页、文件/共享内存和 fork COW。
`Interpreter::use_native_gadgets` 供内部测试切回原路径；`native_gadget_instructions` 用来
证明测试确实经过汇编。公开 C API 和配置布局没有变化。

Python 对照入口为 `benchmarks/compare_gadgets.py`。两边运行同一个 100×100 的数学循环，
必须输出校验和 1267865 且没有未支持 syscall；交替执行旧、新 CLI，丢弃预热轮后比较循环
耗时中位数。脚本同时保留 wall、runtime stats 与二进制 SHA-256。示例：

```sh
python3 hosts/ios/linux-runtime/benchmarks/compare_gadgets.py \
  --baseline /path/to/baseline/operit-arm64 \
  --candidate /path/to/candidate/operit-arm64 \
  --root /path/to/rootfs-arm64 --warmup 1 --repeats 5 \
  --output /path/to/python-comparison.json
```

最新已执行结果和证据路径见 [交付状态](delivery-status.md)。macOS CLI 上的数值不能直接
当作 iOS 设备的性能；本轮按用户要求仅做 macOS 命令行验证。
