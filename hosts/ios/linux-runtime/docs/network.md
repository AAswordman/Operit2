# Linux socket、事件等待与真实网络

本页记录实现边界。最新 network 测试失败 / 修复、Node / Python 报告与待跑结果统一见 [交付状态](delivery-status.md)；历史网络探针不能为当前源码树背书。

## 模块与生产调用

`src/linux/network/` 处理 Linux / Darwin 地址、选项、errno、消息控制数据、resolver 与每 VM 的 UNIX 命名空间。`syscall_socket*.cpp` 处理 guest 内存、FD、部分完成与阻塞等待，`syscall_poll.cpp` / events/epoll 处理 readiness。中央 `syscalls.cpp` 有 socket 与 batch 消息路由，相应源已在 CMake。

数据通过真实宿主 socket 收发；guest Python / Node 及其发行版库执行协议、HTTP 和 TLS。runtime 不伪造服务器响应或直接替换 guest TLS 实现。

| 范围 | 已有实现 / 接线 | 边界 |
|---|---|---|
| IPv4 / IPv6、TCP / UDP、UNIX | socket / pair、bind / listen / connect / accept / accept4、名称 / shutdown | 不等于全部地址族 / socket 类型 |
| send / receive | sendto / recvfrom / sendmsg / recvmsg、guest iovec / cmsghdr 转换、部分完成 | control 类型与 flags 按具体支持检查，不支持返回错误 |
| batch message | sendmmsg / recvmmsg，中央 269 / 243 → socket message | 已有执行体，不能继续写成整体待实现；新 timeout / signal / 部分完成组合待验 |
| SCM_RIGHTS | 真实 native FD 传递；已知 guest UNIX peer 可保留共享对象身份 | 外部 UNIX peer 不能承载所有 guest 虚拟对象语义；运行期间引用环回收仍有边界 |
| credentials | guest UNIX peer 的 SCM_CREDENTIALS / SO_PASSCRED 路径和身份检查 | 宿主 PID 不能冒充 guest PID；非已知 guest peer 的凭据传递存在明确拒绝路径 |
| options / errors | 常用 SOL_SOCKET / TCP / IP / IPv6，SO_ERROR 的 Linux errno 转换，guest 超时 | 全部 socket options / ancillary / 错误队列尚未覆盖 |
| readiness / blocking | ppoll / pselect6 / epoll、临时 mask、非阻塞 connect、调度器等待 | FD 关闭 / 重用、信号中断与对象回收需要生命周期回归 |
| datagram / stream 边界 | UDP MSG_TRUNC / FIONREAD；stream MSG_WAITALL 分段、EOF、部分数据超时 | 带 control / peek / waitall 的全部组合未完成验收 |

宿主 socket 保持非阻塞，guest O_NONBLOCK 独立保存。阻塞调用通过 Scheduler 重试并持有底层对象，避免等待时数字 FD 被复用后误操作新对象。路径 UNIX socket 用私有短目录别名避免 root 前缀耗尽宿主 sun_path，抽象地址按 VM 隔离；仍受宿主路径长度及身份模型限制。

## DNS 与 TLS

rootfs 或绑定源已有 `/etc/resolv.conf` 时使用其内容；缺失时可提供基于宿主 resolver 的只读虚拟文件，不修改外部 root，也不硬编码公共 DNS。该回退不等于全部 Apple 按域 DNS 路由策略。

外部探针通过 guest 原版库进行证书链验证、TLS 和 HTTP。历史报告含实际公网结果，最新证据与适用代码范围见交付状态。公网测试单独启用，以区分本地兼容性和外网条件。

以下为在仓库根目录执行的方法，本轮文档核对未运行：

```sh
python3 hosts/ios/linux-runtime/tests/probe_python.py \
  --runtime tools/ios-runtime/arm64/build/host/operit-arm64 \
  --root tools/ios-runtime/arm64/build/node-rootfs \
  --output tools/ios-runtime/arm64/build/python-probe.json --external-network
```

Node 探针也支持 `--external-network`，测试集合及返回码规则见 [Node 专题](node-probe.md)。网络子项通过不能掩盖 Worker 或其他子项失败。

## 验证入口

入口包括 `tests/network.cpp`、Python 的 `network.py` / `network_external.py`、Node 的 `network.cjs` / `network_external.cjs`。入口存在、源已构建和基础 TCP 通过是不同证据；消息 batch / credentials 新增路径及当前 accept 修复仍需对应运行。完整 Linux 网络 ABI 的缺项列于 [功能矩阵](completeness.md)。
