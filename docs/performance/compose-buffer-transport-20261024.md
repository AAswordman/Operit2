# Compose 二进制所有权传输验证（2026-10-24）

JS 在构造实例、归一化动态参数时直接写入最终布局缓冲区。提交阶段整理地址表并封口；Native Host 脱离 V8 ArrayBuffer，保留 backing store，再通过 CoreBuffer 和 CoreLink 附件传给 Flutter。Flutter 使用索引视图读取参数。生产发送链路不再构造布局响应对象、递归转换布局到 serde_json::Value、反序列化布局事务或把布局重新编码进 MessagePack。

普通工具结果继续使用结构化值；Compose 只接受二进制帧。有限 CLI/桌面快照显式读取同一个格式，不参与会话转发。

## 测量口径

Windows 原生 V8，Debug DLL，电源方案“平衡”。使用相同的 512 行聊天侧边栏数据，展开后为 13,351 个逻辑节点、6,658 个发送实例。依次折叠/展开 16 次，取 8 次展开样本，P50/P95 使用 nearest-rank。初次加载单独记录。

对比值来自此前已完成的平衡模式实测；本次没有重新运行旧基线，因此不是同一进程交替 A/B。完整命令计时包括原生执行及 CoreLink 往返，不包含随后测试侧的节点投影、Flutter 控件构建、布局和绘制。各阶段分位数不能相加得到总耗时分位数。

| 展开 P50 | 此前结构化字段链路 | 本次缓冲区链路 |
| --- | ---: | ---: |
| JS 构造 | 49 ms | 105 ms |
| 事务整理 | 8 ms | 4 ms |
| JS 响应发送 | 309 ms | 0 ms |
| 完整命令往返 | 657.522 ms | 115.827 ms |

发送 P95 为 1 ms。JS 阶段使用毫秒计时，0 ms 不代表没有成本。Rust Instant 记录的 V8 → Rust 参数接收/所有权转交 P50 为 0.014 ms。展开帧的有效布局缓冲区为 2,729,376 字节。

完整命令 P50 缩短约 82.4%，约为此前的 5.68 倍速度。构造阶段增加的开销包含实际缓冲区写入，已经计入上表，没有隐藏到发送计时之前。

## 复杂度边界

原生发送步骤转交已有存储的所有权，不遍历布局内容。构造、参数归一化、列表成员关系检查、Flutter 参数校验和布局仍然随实际工作量增长。任意大型 actionResult 的写入和读取也有成本。Web Host 使用相同协议，但 JS 存储进入 Wasm 线性内存时仍有一次字节复制；网络传输仍支付字节传输成本。

原始结果：`.tmp/sidebar-program-native-v8-buffer-results.json`；比较结果：`.tmp/sidebar-program-native-v8-buffer-comparison.json`；完整日志：`.tmp/sidebar-program-native-v8-buffer.log`。生成器和原生 Flutter 探针分别为 `.tmp/sidebar_program_buffer_generate.mjs` 与 `apps/flutter/app/.tmp/sidebar_program_native_v8_buffer_test.dart`。
