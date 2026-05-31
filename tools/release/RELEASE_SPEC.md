# Operit2 Release Specification

本文档定义 Operit2 全量更新使用的 GitHub Release 规范。更新检查与下载逻辑必须按这里的规则严格匹配 release asset 名称。

## Release Tag

正式 release tag 使用：

```text
v{major}.{minor}.{patch}+{build}
```

示例：

```text
v1.0.0+1
v1.0.1+2
v1.1.0+10
```

版本比较顺序：

```text
major -> minor -> patch -> build
```

比较时允许 tag 带 `v` 前缀；`+build` 参与比较。

## Asset Name

全量更新包统一使用：

```text
operit2-{product}-{platform}-{arch}.{ext}
```

字段含义：

```text
product  产品形态
platform 运行平台
arch     目标架构或 Android ABI
ext      包格式
```

更新检查必须按完整文件名严格匹配，不按别名、包含关系或拆词猜测。

## Product

允许的 `product`：

```text
app
cli
```

`app` 表示 Operit2 本体。

`cli` 表示 Operit2 CLI/TUI。

CLI/TUI 更新只查找 `operit2-cli-*` asset。本体更新只查找 `operit2-app-*` asset。

## Platform

允许的 `platform`：

```text
windows
linux
macos
android
```

## Desktop Arch

桌面端允许的 `arch`：

```text
x86_64
aarch64
```

## Android ABI

Android 端 `arch` 使用 Android ABI 标准名：

```text
arm64-v8a
armeabi-v7a
x86_64
```

因为 `arm64-v8a` 和 `armeabi-v7a` 自身包含 `-`，代码不得通过简单按 `-` 拆分 asset 名称来解析 Android ABI。

## App Assets

Operit2 本体全量更新包：

```text
operit2-app-windows-x86_64.zip
operit2-app-windows-aarch64.zip

operit2-app-linux-x86_64.tar.gz
operit2-app-linux-aarch64.tar.gz

operit2-app-macos-x86_64.tar.gz
operit2-app-macos-aarch64.tar.gz

operit2-app-android-arm64-v8a.apk
operit2-app-android-armeabi-v7a.apk
operit2-app-android-x86_64.apk
```

## CLI Assets

Operit2 CLI/TUI 全量更新包：

```text
operit2-cli-windows-x86_64.zip
operit2-cli-windows-aarch64.zip

operit2-cli-linux-x86_64.tar.gz
operit2-cli-linux-aarch64.tar.gz

operit2-cli-macos-x86_64.tar.gz
operit2-cli-macos-aarch64.tar.gz
```

Android 默认不发布 CLI/TUI asset。

## Matching Rules

更新检查根据当前产品形态、平台、架构生成唯一目标文件名，然后在 GitHub Release assets 中严格查找这个名称。

CLI/TUI 示例：

```text
windows x86_64  -> operit2-cli-windows-x86_64.zip
windows aarch64 -> operit2-cli-windows-aarch64.zip

linux x86_64    -> operit2-cli-linux-x86_64.tar.gz
linux aarch64   -> operit2-cli-linux-aarch64.tar.gz

macos x86_64    -> operit2-cli-macos-x86_64.tar.gz
macos aarch64   -> operit2-cli-macos-aarch64.tar.gz
```

App 示例：

```text
windows x86_64       -> operit2-app-windows-x86_64.zip
windows aarch64      -> operit2-app-windows-aarch64.zip

linux x86_64         -> operit2-app-linux-x86_64.tar.gz
linux aarch64        -> operit2-app-linux-aarch64.tar.gz

macos x86_64         -> operit2-app-macos-x86_64.tar.gz
macos aarch64        -> operit2-app-macos-aarch64.tar.gz

android arm64-v8a    -> operit2-app-android-arm64-v8a.apk
android armeabi-v7a  -> operit2-app-android-armeabi-v7a.apk
android x86_64       -> operit2-app-android-x86_64.apk
```

## Download Requirements

每个全量更新 asset 必须满足：

```text
Content-Length > 0
HTTP Range request returns 206
```

全量更新下载使用 6 线程 Range 下载。

## Current GitHub Source

Operit2 正式全量更新源：

```text
https://api.github.com/repos/AAswordman/Operit2/releases?page=1&per_page=1
```
