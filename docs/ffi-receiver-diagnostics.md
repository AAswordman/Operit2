# Core FFI 接收异常诊断日志

用于排查 `List<dynamic> is not a subtype of Uint8List` 等接收异常。诊断版本标记为 `diagnostics=ffi-v1`，保留现有 ABI v4、批次格式、类型检查和关闭连接行为。

## Dart 接收侧

`CoreFfiTransport` 的原始错误、堆栈和连接状态继续记录。新增字段：

- `nativePort`：用于筛选同一原生端口的发送记录。
- `receiveBatch`、`receivedFrames`：该连接已进入的批次数、已成功处理的帧数。失败帧不计入后者。
- `stage`、`failingSlot`：失败时的处理阶段和正在转换的 envelope 槽位。`metadata / 0` 对应原来的 `envelope[0] as Uint8List`；`receipt` 对应附件字段；`decode`、`dispatch` 阶段槽位为 `-1`。
- `lifecycle`：错误发生时 Flutter 的生命周期状态；尚无绑定时为 `unavailable`。
- `connectionAgeMs`、`receiveGapMs`：诊断计时器启动至今的时长、距前一次接收回调的间隔。它们不能单独证明系统挂起了进程。
- `nativeApi`：当前 Dart Native API 主次版本。

先记录失败帧，再记录批次摘要，减少异常对象被后续输出淹没的可能：

- `ffi_rx_slot`：失败帧前 33 个槽位的类型、长度、对象 identity。若失败槽位超过该上限，额外输出 `ffi_rx_failing_slot`。
- `ffi_rx_frame`：批次前 128 帧的形状和 metadata 摘要。
- `sampled / byteValues / lists / types`：普通 List 前 16 项的统计、前 8 项的类型和子列表长度。仅当采样项全部是 0–255 的整数时，尝试把前 9 项解释为协议头，输出 `possibleKind / possibleId`；不会用它继续处理消息。
- `aliases`：通过 `identical` 确认对象是否就是批次、当前帧或其他帧槽位。每次最多扫描 128 帧、每帧 33 槽、总计 2048 项，保留最多 16 个匹配。没有匹配只代表在扫描范围内没有发现；`identityHashCode` 本身可能碰撞，不作为相同对象的证据。

不会递归展开列表，因此自引用对象不会导致无限遍历。诊断异常会记录为 `ffi_rx_diagnostic_error`，不会替换原始接收错误。

## Rust 发送侧

`DartPort::sendBatch` 在调用 `postCObject` 前保存标量快照，正常发送不会逐条写日志。进程内最多保留最近 32 批，每批最多 128 帧，以及每帧 metadata 和第一个附件 receipt 的描述符。记录所有已采样帧的槽位类型检查结果和槽位指针匹配计数。快照不保留业务数据或其所有权，不在 post 返回后解引用数据、peer 或描述符地址。

Dart 通过现有同步 `reportFailure` 上报异常时，在日志输出前复制同端口的快照，然后输出：

- `ffi_tx_history`：容量、采样上限、架构、指针宽度、DartObject 大小及预期类型枚举。
- `ffi_tx_batch`：全进程 post 序号、端口、Unix 毫秒时间、post 返回值、根描述符/数组地址、批次大小和采样帧数。`accepted=None` 表示快照复制时 post 尚未登记返回结果，不代表拒收。
- `ffi_tx_frame`：帧索引、envelope 和指针数组地址、槽位数、协议头 kind/id、`firstBadSlot`、`pointerMismatches`、metadata 和首个 receipt 的描述符地址/type/typedType/length/data/peer。

类型枚举预期为 array=6、external=13、Uint8=2。`firstBadSlot=None` 和 `pointerMismatches=0` 只表示快照采集时该部分符合预期，不能证明后续序列化全程正确。

快照历史是全进程共享的有界缓存，其他端口的发送也会占用容量。若消息排队很多或其他端口活跃，失败批次可能已被覆盖。原生 post 序号与 Dart `receiveBatch` 不是同一计数器；需要结合端口、批次大小、帧索引、协议头和时间进行对照。原生地址与 Dart identity 也不是同一地址空间标识，不能直接比较数值。

## 描述符身份与地址分配

出现 `aliases=[frame[5],frame[8].slot[0]]` 时，表示第 8 帧的 metadata 槽与第 5 帧数组是同一个 Dart 对象，已经发生引用串位。这与将字节数组解码成普通整数 List 的情况不同。

Dart 3.11.5 的 [`ApiMessageSerializer`](https://github.com/dart-lang/sdk/blob/3.11.5/runtime/vm/message_snapshot.cc) 将原生 `Dart_CObject*` 直接转换为 `ObjectPtr`，以 [`WeakTable`](https://github.com/dart-lang/sdk/blob/3.11.5/runtime/vm/heap/weak_table.h) 记录对象身份。[`ObjectPtr` 的比较实现](https://github.com/dart-lang/sdk/blob/3.11.5/runtime/vm/tagged_pointer.h) 在指针压缩开启且最低位为 0 时只比较低 32 位；正常对齐的原生描述符地址会进入该分支。相隔 4 GiB 整数倍的不同原生描述符可能被误判为同一引用。在常规小表中，其哈希槽也相同。

`DartPort::sendBatch` 现在把根描述符、所有帧数组描述符、metadata 和附件槽描述符放入同一块小于 4 GiB 的连续分配，确保这些描述符的地址低 32 位彼此不同。先完成描述符构造，再建立指针数组，发送期间不扩容；正文、附件、receipt 数据和释放协议保持原有所有权，不复制二进制负载，也不在接收侧接受错位的 List。

原生回归检查覆盖 1、12、128 帧的完整对象图，验证地址跨度、低 32 位身份、槽位类型与 metadata 内容；该检查在原发送端失败，在连续分配后通过。Flutter 测试额外通过真实 VM 端口发送 256 批、每批 12 个无附件事件，检查 3072 个值的顺序和完整性。

这些检查验证发送端的分配约束和本地传输，不等于复现了 Android 上的引用串位。本机 VM 上，强制构造低 32 位相同地址的实验没有出现串位。确认某一次故障是否由该机制触发，仍应将同一批的 `ffi_tx_frame index=8` 的 metadata `object` 地址与 `index=5` 的 `envelope` 地址对照：需要看到完整地址不同而低 32 位相同。仅有 Dart 接收日志时，不能证明这一地址条件成立。

## 收集方式与范围

需要重新构建并安装包含此次 Dart 和 Rust 改动的应用，单独热重载 Dart 不能增加原生发送快照。复现后保留同一错误附近完整的 `CoreFfiTransport` 记录，包括 `ffi_rx_*` 和 `ffi_tx_*`，不要只截取异常堆栈。

日志不输出聊天正文、脚本或附件内容，只输出协议头、浅层类型、长度、结构字段和本地原生地址。正常路径增加计数、时间采样、快照分配和短时互斥锁开销；详细扫描、字符串格式化和批量输出在异常时进行。验证范围见上文；未构建安装 Android 应用或进行设备复现。
