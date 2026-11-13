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

## 收集方式与范围

需要重新构建并安装包含此次 Dart 和 Rust 改动的应用，单独热重载 Dart 不能增加原生发送快照。复现后保留同一错误附近完整的 `CoreFfiTransport` 记录，包括 `ffi_rx_*` 和 `ffi_tx_*`，不要只截取异常堆栈。

日志不输出聊天正文、脚本或附件内容，只输出协议头、浅层类型、长度、结构字段和本地原生地址。正常路径增加计数、时间采样、快照分配和短时互斥锁开销；详细扫描、字符串格式化和批量输出在异常时进行。本次没有运行测试或复现程序。
