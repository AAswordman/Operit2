# Rust–Dart ABI v2 共享二进制

原生 Flutter Core 通信只使用 ABI v2。v1 的同步 C 调用导出、`OperitByteBuffer` 返回结构、原生 watch 轮询队列及对应执行适配已删除。Dart 在读取 session 和函数地址前要求描述符 `version == 2`。

完整函数表、操作编号、消息布局与检查命令见 [Flutter Core FFI transport](../apps/flutter/native/operit-flutter-bridge/FFI.md)。

## 调用链与边界

Dart 生成客户端构造 `CoreCallRequest`，经 `ProxyCoreRuntimeBridge`、`FfiCoreProxy`、`FfiCoreTransport` 进入 Rust 的 `FfiTransport`。Rust 使用当前 Host 安装的任务调度器执行请求，经 `LocalCoreProxy` 和生成分发器调用业务对象；结果通过 Dart native port 异步返回。

`MethodChannel('operit/runtime')` 负责 `connectCoreFfi`、启动配置、存储根目录和应用生命周期。平台 Host 拥有 Runtime；FFI session 保留其 `Arc` 引用，不创建另一个 Core Runtime。每个连接独立管理请求编号、watch 和 push。

平台选择保留在 Host 与 ABI 分发边界。Web 通过 JS interop、Runtime Worker 和 Rust WASM 使用相同的 Core 请求、响应与事件协议；其载体输出内联 MessagePack 二进制值，不传递原生共享指针。这个浏览器载体属于独立平台边界。

## 提交与内存所有权

1. Rust `CoreValue::Bytes` 使用 `CoreBytes`（`bytes::Bytes`）。克隆和切片保留底层所有权，不复制二进制内容；同地址、同长度的共享值可以直接判等。
2. `CoreLinkMessage` 分别保留元数据和二进制 span。普通 Dart `Uint8List` 首次提交时复制到 Host 分配的原生块，再 seal 为不可变存储。`FfiCoreTransport.shareBytes` 返回可重复提交的只读共享视图；Rust 返回的共享视图也可直接提交。
3. `submit(session, operation, requestId, metadataPointer, metadataLength, handlesPointer, handleCount)` 接收一个完整输入分配。该分配包含控制字节、原生指针对齐 padding 和 handle 表。Rust 同步接管整个分配并 retain 所有二进制块，再通过当前 Runtime 的 Host 调度器执行请求。Dart 保持源视图存活直至 FFI 调用返回。
4. `allocate(totalLength)` 的结果由 `submit` 接管。只有尚未提交的分配由 Dart 调用 `free(pointer, totalLength)` 释放。调度任务保留独立 session 引用。
5. MessagePack 编解码作用域只包围同步编解码，不跨 `await`。二进制引用的解析只访问当前消息已保留的 buffer 表。

## 共享引用与端口信封

MessagePack ext `0x42` 的内容为三个大端 u64：buffer 表索引、偏移、长度。内容长 24 字节，连同 MessagePack 前缀共 27 字节。解码器校验扩展类型、长度、索引、范围和整数溢出。

Rust 回复的端口信封为：

```text
[controlFrame, handle, receipt, address, length, ...]
```

`controlFrame` 是外部 typed data，Dart 暴露其只读视图。其头部为 1 字节消息类型及 8 字节小端有符号 FFI 请求编号，随后是 MessagePack 内容：

| 类型 | 内容 |
| --- | --- |
| 0 | 调用结果或操作确认 |
| 1 | watch 事件 |
| 2 | watch 错误 |
| 3 | watch 结束，内容为空 |

watch 消息沿用打开订阅时的 FFI 请求编号。该编号与 MessagePack 请求内部的 Link request ID 分别管理。

每个二进制条目携带存活 handle、1 字节 external typed data receipt、原生地址和长度。receipt 的 native finalizer 拥有并释放该 handle。Dart 在接收期间保持整个信封存活，新建视图时同步 retain，并绑定 `asTypedList` native finalizer。

Dart 按地址和长度复用仍存活的共享视图。弱引用表不拥有块，GC 清理对应条目。新视图按真实长度计入 VM 外部内存；重复传递相同的存活视图只增加小型凭据。

端口拒收时立即释放该次投递的全部原生所有权。已投递凭据和视图由 VM finalizer 释放。关闭连接不使已返回的共享视图失效；外部 typed data 的最终回收取决于 VM GC。

## 信任边界与流生命周期

handle 是同一进程内 Host 提供的存活指针。Dart 校验共享视图的 Host domain，Rust 要求 seal 后才能 retain。元数据中的索引、偏移和长度必须通过 codec 校验。数据 seal 后不可修改；业务需要修改时显式创建拥有的可写副本。

watch 事件可以早于打开订阅的确认到达。Dart 在提交前安装接收控制器，按源顺序交付事件；源错误和 panic 通过流错误传递。取消订阅释放该连接拥有的 watch，不影响其他连接。

push 使用打开、顺序发送和关闭三个阶段。Dart 串行提交 item，Rust 校验连接所有权和递增序号，拒绝重复 push ID 或错误序号。

## 性能边界与使用

- 对已经共享的单个二进制块，引用编码、传递、解码与共享切片相对内容大小为 O(1)。多个二进制块和结构字段仍需逐项处理。
- 首次把 Dart 堆字节迁入原生存储、生成或读取全部内容、字符串编码、转换为业务 `Vec<u8>` 仍为 O(n)。
- Dart 使用 `sliceCoreLinkBytes` 保留共享身份；自行创建的标准 typed-data view 不保证携带 codec 共享记录。`Uint8List.sublist` 创建副本。
- 原生共享视图不可修改。网络和 Web 的内联载体仍处理实际二进制内容，不能把原生共享传递的复杂度外推到这些边界。

## 验证

架构检查覆盖 v1 导出与轮询队列的移除、v2 描述符约束、Host 调度器归属，以及浏览器接口的保留。Dart codec 测试覆盖共享引用范围与 Flow 重建，地址测试覆盖原生指针位宽与地址位的保留。

`ffi_core_transport_test.dart` 使用真实 FFI 函数指针和 Dart native port，覆盖共享视图、并发切片、缓冲区生命周期、连接关闭、取消、错误与 GC。该测试运行时会编译 Rust native fixture；日常无需构建应用的检查命令见 FFI 文档。跨平台集成效果需在对应平台 Host 上验证。
