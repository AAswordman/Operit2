# Rust–Dart ABI v2 共享二进制

用户已批准升级本机 ABI。生产本机路径统一使用 v2：控制消息携带共享引用，二进制内容留在原生存储中。Call、Push、Watch/Stream、Flow 的二进制值使用同一所有权契约。

## 传递与所有权

1. Rust `CoreValue::Bytes` 使用 `CoreBytes`（`bytes::Bytes`）。克隆、切片只保留所有权，不复制内容；同地址、同长度的值比较直接判等，Flow 不扫描未改变的共享块。
2. `CoreLinkMessage` 将元数据与二进制 span 分开保留。普通 Dart `Uint8List` 首次提交时复制到 host 分配的原生块并 seal；`FfiCoreTransport.shareBytes` 返回可反复提交的只读共享视图。Rust 返回的视图也可直接提交。
3. native submit 接收控制字节与 handle 表，返回前同步 retain 所有二进制块，再交给固定 16 线程 scheduler。编解码作用域只包围同步编解码，不跨 await。
4. MessagePack ext `0x42` 的 24 字节内容为三个大端 u64：表索引、偏移、长度。每个引用连前缀共 27 B。解码检查扩展类型、长度、索引、范围及整数溢出；索引只能访问当前消息已保留的表。
5. 回复端口信封为 `[controlFrame, handle, receipt, address, length, ...]`。controlFrame 是带原有 9 字节头的只读外部字节视图；每个 receipt 是 1 字节 external typed data，其 native finalizer 持有并释放该 handle。
6. Dart 在接收期间保持整个信封存活，按地址和长度复用仍存活的共享视图；新视图同步 retain 后绑定 `asTypedList` native finalizer。弱引用表不拥有块，GC 清理其条目。每个新视图按真实长度计入 VM 外部内存，相同存活视图的重复传递仅计入小型凭据，避免整块重复计账引发 GC。
7. 被拒绝的端口消息立即释放全部原生所有权；已投递视图由 VM GC/finalizer 释放。关闭连接不使已返回的视图失效。isolate 同组退出后仍可能需要 GC 才完成 external typed data 释放。

## ABI 表与信任边界

函数表版本为 2，保留 session / attach / allocate / free / submit / release，增加 bufferAllocate / bufferData / bufferSeal / bufferRetain / bufferRelease。submit 参数增加 handle 表及数量。输入分配包含控制字节、指针对齐 padding、handle 表；Rust 接管整个输入分配。

handle 是由同一 host 提供的存活原生指针，属于可信进程内 FFI 契约，不是来自网络的任意地址。Dart 校验 host domain，Rust 要求 seal 后才 retain；不提供伪造地址的安全解引用或失效句柄恢复。线上的引用索引/偏移/长度由 codec 严格校验。数据 seal 后不可修改；需要修改的业务必须显式创建拥有的可写副本。

平台入口继续由 host 分发。rs-transport-rs 与 Wasm 的字节载体按其边界输出普通内联 MessagePack；本机生产 FFI 只接受 v2。

## 复杂度与使用

- 已共享的一块二进制：引用编码、传递、引用解码与共享切片相对字节数 O(1)。多个二进制块/结构字段仍需逐项处理。
- 首次生成或把 Dart 堆字节迁入原生存储、读取全部内容、字符串编码、转换为业务 `Vec<u8>` 都仍为 O(n)。这里没有宣称业务内容处理变为 O(1)。
- Dart 使用 `sliceCoreLinkBytes` 保留共享身份；任意自行创建的标准 typed-data view 不保证携带 codec 的共享记录。`Uint8List.sublist` 明确创建副本。
- 原生字节视图是不可修改的。网络传输仍需处理实际数据，不能将进程内共享结果外推到网络。

## 验证与复现

`tools/performance/run_core_link_bench.py --shared` 使用实际 Dart transport/codec、Rust codec、DartPort、16 线程 scheduler；以 echo 替代 Core 业务，Rust release + Dart AOT。每次 Call 重新编码控制消息，完整内容比较和原生地址核对在计时外进行。请求元数据三档均 50 B，回复均 29 B。

Windows Ryzen 9 7940HX 本次测量：

| 共享块 | Call p50 | Push+ACK p50 |
| --- | ---: | ---: |
| 64 KiB | 15.8 µs | 13.7 µs |
| 1 MiB | 15.6 µs | 14.1 µs |
| 16 MiB | 15.2 µs | 13.5 µs |

第二次独立进程复测 Call p50 为 17.7 / 17.8 / 21.3 µs，记录在 `windows-shared-o1-repeat.json`。这是微秒级调度波动；三档仍只传固定大小元数据，地址核对全部一致。

详细环境、逐轮结果、首次迁入成本与源码/二进制哈希见 `tools/performance/core_link_bench/results/windows-shared-o1-final.json` 和同名 Markdown。

检查覆盖引用范围、嵌套作用域与 panic 恢复、同地址往返、并发切片、Flow 重建保留引用、不可修改视图、连接关闭后存活、取消/错误、拒收释放以及 VM GC 后原生分配归零。实际 production bridge 通过 cargo check；跨平台代码结构已保持，本次性能数据仅实测 Windows。
