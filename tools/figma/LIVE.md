# 实时运行与编辑连接

这是连接真实 Operit 的 Web 客户端初版。它持续接收当前界面，把点击、滚动、输入及受支持的设计修改交回真实 Flutter / 插件渲染器。它需要同一台电脑上的 debug Operit；当前还不能作为一份自包含的完整软件交给设计同学。

## 启动

在 VS Code 中选择 `Operit2: Figma design`，或在仓库根目录执行：

```sh
cd apps/flutter/app
.fvm/flutter_sdk/bin/flutter run -d macos \
  --dart-define=OPERIT_DESIGN_BRIDGE=true \
  --vmservice-out-file=/tmp/operit-design-vm.txt
```

待应用启动，在另一个终端回到仓库根目录：

```sh
npm --prefix tools/figma run build:live
node tools/figma/live.mjs --vm-service-file /tmp/operit-design-vm.txt
```

打开命令输出的本机连接地址。地址中的口令保留在 URL 片段中，客户端连接成功后清除地址栏片段；服务仅监听 `127.0.0.1`，请求使用 Authorization 头。请勿把完整连接地址提交到仓库、上传到 Figma 代码文件或公开分享。

进入客户端后可以直接操作软件；点击“选中编辑”选择控件，右侧仅显示真实运行端支持的属性。“撤销该控件修改”恢复业务最新值。“保存设计修改”导出带来源选择器的修改清单，尚不自动改写原始 Dart 或插件源文件。

## Figma 候选项目

`build:live` 生成：

- `dist/live/index.html`：本机预览客户端。
- `dist/live-code-layer/`：不含账号、业务数据和连接口令的独立 Web 前端项目。
- `dist/operit-live-code-layer.zip`：上述项目的候选导入包。

候选项目包含入口、启动和构建脚本，以及 Figma 要求的四个可执行生命周期脚本（`.figma/make/install`、`dev`、`deploy-preview`、`deploy`）。ZIP 保留脚本的 Unix 执行权限。开发服务器读取 `PORT` 和 `FIGMA_DEV_SERVER_HOST` 环境变量。它不包含 Operit 原生程序。导入真实 Figma Code layer 后，仍需在同一台电脑上运行 Operit，并在客户端粘贴本机连接地址。

远程预览来源需要明确允许。确认真实预览 Origin 后重启连接服务：

```sh
node tools/figma/live.mjs --vm-service-file /tmp/operit-design-vm.txt \
  --allow-origin https://实际预览域名
```

该参数只接受精确 Origin，不接受通配符。Figma 的 CSP、HTTPS 页面到本机 HTTP 的访问、浏览器本地网络权限和沙箱限制都需要在真实容器中验证；不能把浏览器 fixture 测试视为 Figma 验收。

## 已验证的 Figma 导入阶段

2026-10-30，通过 Figma 官方远程 MCP 的 `send_code_to_figma`，实际创建了 Make 文件并导入生成的项目。服务返回 `status: imported`；在已登录的 Figma Make 编辑器中，预览显示 `Running`，文件面板中可打开 `index.html`，编辑器显示可写输入区。当前账号未开通 Design Code Layer，因此本次使用独立 Make 文件。

这些结果只证明真实 Figma 中的源码导入和客户端启动。真实 Operit 连接、完整软件交互、Figma 原生可视化编辑与原始源码回写仍需分别验收，不能以导入回执替代。

## 验证

```sh
npm --prefix tools/figma run typecheck
npm --prefix tools/figma run test:live
npm --prefix tools/figma run build:live
npm --prefix tools/figma run test:live:browser
```

浏览器测试需要安装 Playwright 和 Chromium，也可通过 `OPERIT_PLAYWRIGHT_MODULE` 指定 Playwright 模块，通过 `OPERIT_BROWSER_EXECUTABLE` 指定现有 Chrome。

`test:live` 使用受控 VM / HTTP fixture，验证方法选择、事件传递、请求顺序和访问限制。`test:live:browser` 使用界面 fixture，验证真实浏览器内的点击、输入确认、焦点保留、编辑及撤销。两者都不证明全软件覆盖或真实 Figma 兼容。

## 尚未满足的交付条件

- 真实 Figma 文件内的连续运行和原生 Point and edit 仍待验收。
- 客户端目前转发 tap、scroll、input；拖动、键盘快捷键、长按等交互还没有完整实现。
- 只对运行端声明的属性提供编辑。自定义绘制内部、跨域 iframe 等不能逐控件编辑。
- 有稳定来源标识的控件可跨重建保留修改；没有标识的控件只在其存活期间保留。
- 设计修改自动回到原始业务源文件尚未实现。

整软件交付标准仍以 [INTERACTIVE_APP.md](INTERACTIVE_APP.md) 为准。
