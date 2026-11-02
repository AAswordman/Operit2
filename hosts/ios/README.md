# iOS host

## ARM64 Linux runtime

独立运行时位于 [linux-runtime](linux-runtime/README.md)，按 CPU、内存、ELF、文件系统、Linux ABI 与固定组件加速分层。Flutter Runner 的终端通过 Rust 桥接调用 `Arm64TerminalBridge.mm`，使用原版 Alpine AArch64 软件包。构建与 rootfs 工具位于 `tools/ios-runtime/arm64/`；旧 x86 后端已从产品构建和终端路由移除。

## Terminal storage and paths

The ARM64 installer imports the bundled `arm64-rootfs.tar.gz` once into
`Library/Application Support/operit-arm64/rootfs-arm64` in the application data container.
Subsequent launches reuse it. Files in `/root`, shell configuration, and software
installed into `/usr` survive process restarts and app updates that preserve app
data. Removing app data also removes this filesystem.

Host directories are shared through the runtime bind mounts **at their original absolute paths**,
matching Android's same-path bind mounts. The Documents directory and Operit2
application-support directory are available to every shell; a host working
directory can also be mounted on session creation. The shell and native file
tools read and write the same files immediately. Mounts are reestablished on
launch; no workspace copy or hash-based MCP path is involved. Linux-only paths
such as `/root` belong to the persistent Alpine filesystem.

On a device, host paths start with the app's `/var/mobile/...` container path;
simulators use `/Users/...`. iOS can change the container UUID during an update.
The native bootstrap channel restores persisted runtime/workspace roots to the
current container. Scripts should use `$HOME` for terminal-owned files and the
current absolute workspace path supplied by the host for shared files.

## Serial transport

Core opens serial endpoints through `SerialPortHost`. The iOS Host currently
reports that no native serial accessory provider is installed. Implementing an
accessory provider and registering it in the host manager enables the existing
Link framing and pairing protocol without platform branches in Core. This does
not advertise unrestricted USB/UART access on iOS.
