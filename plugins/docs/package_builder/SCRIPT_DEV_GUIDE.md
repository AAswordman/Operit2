# 脚本开发指南

## 1. 简介

本文档旨在为开发者提供关于如何编写、构建和维护自动化脚本的全面指南。**这些脚本旨在作为强大的工具，被导入到 Operit AI 智能助手中，并由 AI 根据用户指令进行调用，从而极大地扩展应用的功能边界。** 这些脚本基于一个强大的框架，提供了一系列用于设备控制、UI自动化、网络请求和文件操作的工具。

脚本主要使用 **TypeScript** 编写，以利用其强大的类型系统，但也可以使用原生 JavaScript (ES6+)。

## ToolPkg API 版本与跨平台要求

Operit2 的 ToolPkg API 支持版本为 `2.0.0`；对 `1.0.0` 和 `1.0.1` 的加载支持并不完整。Operit1 完整支持 ToolPkg API `1.0.0` 和 `1.0.1`，这两个版本主要面向 Android，是旧版 API 形式。

在 Operit2 创建 ToolPkg 时显式填写 `"api_version": "2.0.0"`，接口以随 PackageBuilder 携带的当前版本类型为准。不能通过仅改 manifest 版本号把旧 Android 插件视为已完成迁移；需要检查实际接口与平台行为。`api_version` 是 ToolPkg 清单字段，不是单文件脚本的元数据字段。

ToolPkg API `2.0.0` 面向多平台开发，基本所有公共接口都通过统一 host 能力提供跨平台兼容。常规功能直接使用当前版本接口即可，不必为每个平台重复实现。作者仍需注重跨平台兼容性：遇到平台特异接口时，明确其适用范围，考虑并验证其他平台的安装、界面与核心功能；不能把某个平台的专用能力当作所有平台都具备的接口。

## 2. 快速上手

**📝 最简单的方式：直接编写 JavaScript**

如果你只是想快速编写脚本，不想配置任何环境，可以直接创建 `.js` 文件并编写 JavaScript 代码。不需要安装 TypeScript、不需要编译，只需要一个文本编辑器即可。参考第 4 章的脚本结构 中的代码结构，去掉类型标注即可。

**开发环境与市场发布是两件独立的事：**

*   **在 Operit 项目中开发**：适合修改主项目、内置示例或宿主能力。
*   **维护独立插件项目**：适合需要长期维护源码、构建脚本、版本和 Git 历史的作者。

无论使用哪种开发环境，市场发布都可以选择直接上传当前本地包，或引用作者自己 GitHub Release 中已经上传的资产。

---

## 发布到插件市场

市场发布支持两条路线，它们对应不同的维护方式：

1. **直接发布本地插件**：选择当前已构建的脚本包或 `.toolpkg`，在发布页面选择“直接发布本地插件”，填写市场资料并提交。此路线只要求完成当前包的构建和核对；Operit 会在当前 GitHub 账号的 `OperitForge` 仓库创建或更新 Release，用于保存该发布资产。
2. **引用 GitHub Release 资产**：适合长期维护独立插件仓库的作者。先在自己的仓库构建插件、整理提交和 tag、创建 GitHub Release 并上传最终资产；再在 Operit 中选择同一份本地插件，选择“引用 GitHub Release 资产”，填写仓库链接并选择 Release 与资产名称。

引用 Release 时，Operit 会下载所选资产并与当前本地插件计算 SHA-256 比对；市场服务还会确认该 Release 的创建者就是当前 Operit 登录的 GitHub 账号。Release 正文属于作者自己的版本说明，不需要添加 Operit proof、签名文本或其他平台专用内容。

当用户只希望完成当前版本发布时，AI 只需协助构建最终包并指导其使用发布页面。选择独立仓库路线时，AI 应持续协助维护源码、manifest、构建脚本、依赖清单、`.gitignore`、README、许可证、提交和 Release。公开仓库可以独立分享、协作和获得 star；市场条目只是额外的分发入口，公开程度与许可证必须由作者决定。

---

### 2.1. 当前版本开发入口

从包管理的“快速创作你的插件”开始时，应用准备 PackageBuilder 与 operit_editor，并把需求填入聊天草稿。用户发送草稿后，由 AI 与用户协作开发；不是按钮自动生成或安装成品。

先阅读 [当前版本插件创作流程](./PLUGIN_CREATION_WORKFLOW.md)。通过 `["skill", "show", "PackageBuilder"]` 读取真实 Skill 目录，检查终端 host 与可写文件位置，确定 `<dev_root>`。所有平台都使用现有 host API；不固定到某个平台的下载目录。

独立插件项目放在 `<dev_root>/<package_id>/`，将 Skill 的 `types/` 完整复制到 `<dev_root>/types/`。贡献 Operit 主项目时，脚本源码位于 `plugins/packages/buildin/` 或 `plugins/packages/external/`；不要把 Skill 内的示例副本当作应用已经安装的包。

### 2.2. TypeScript 项目配置

确认当前终端具备可用的 Node.js、包管理器和 TypeScript 编译器后，在包项目中声明开发依赖及构建命令。安装开发依赖只影响项目构建；它不向 QuickJS 注入 Node.js 模块。

```json
{
  "private": true,
  "scripts": {
    "build": "tsc -p tsconfig.json"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
```

`tsconfig.json` 示例适用于以下结构：`<dev_root>/types/` 与 `<dev_root>/<package_id>/src/`。ToolPkg 的 main、子包、UI 等编译入口按项目需要放在 `src/` 下，manifest 路径对应 `dist/` 中的产物。

```json
{
  "compilerOptions": {
    "target": "es2020",
    "module": "commonjs",
    "lib": ["es2020"],
    "rootDir": "src",
    "outDir": "dist",
    "moduleResolution": "node",
    "strict": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "typeRoots": ["../types"]
  },
  "include": ["src/**/*.ts", "../types/**/*.d.ts"],
  "exclude": ["node_modules", "dist"]
}
```

- CommonJS 对应当前运行时模块加载方式；不能把构建时使用的 Node.js 当作脚本执行宿主。
- `lib` 不引入浏览器 DOM；全局 `Tools`、`console` 等声明由随 Skill 携带的类型提供。
- `include` 纳入完整平台声明。`src/` 内三斜线引用的相对路径为 `../../types/index.d.ts`。
- 确认真实编译器可用后执行项目声明的构建命令，例如 `npm run build`。检查退出码与 `dist/` 文件，不根据命令已提交宣称构建成功。

### 2.3. 第一个包的安装与执行

在 `<dev_root>/<package_id>/src/my_new_script.ts` 按第 4 章写入 `METADATA` 和导出函数，构建得到 `dist/my_new_script.js`。单文件 JS 包直接导入编译成品；ToolPkg 按格式指南打包后导入。

以下是系统工具 `execute_cli_command` 的参数数组，路径须替换为当前 runtime FileSystemHost 可读的真实成品路径：

```json
["package", "import", "<artifact_host_path>"]
["package", "enable", "MyNewScript"]
["package", "list"]
["package", "show", "MyNewScript"]
["package", "exec", "MyNewScript:hello_world", "{\"name\":\"世界\"}"]
["log", "package"]
```

名称与导出函数来自第 4 章示例。实际项目使用导入结果与 `package show` 返回的名称。核对 `package list` 的真实启用状态与执行返回内容。同名包重新安装会被拒绝，更新需要按 [创作流程第 5 节](./PLUGIN_CREATION_WORKFLOW.md#5-同-id-的修改与重新安装) 完成明确授权的替换操作。

## 3. 核心概念

在开始编写脚本之前，理解以下几个核心概念非常重要：

### 3.1. 脚本元数据 (METADATA)

每个脚本文件的开头都必须包含一个 `/* METADATA ... */` 注释块。这个块定义了脚本的名称、描述、分类以及最重要的——它所提供的工具。**这块元数据是 Operit AI 理解并调用你所编写功能的唯一途径。AI 会解析 `METADATA` 中的信息，将其作为可用的“工具”呈现给大语言模型（LLM），从而实现通过自然语言指令来执行复杂脚本的能力。**

**示例：**（元数据结构示意；具体示例以本 Skill 的 `examples/packages/` 为准）

```typescript
/*
METADATA
{
    "name": "Automatic_bilibili_assistant",
    "display_name": {
        "zh": "B站智能助手",
        "en": "Bilibili Assistant"
    },
    "description": "高级B站智能助手，通过UI自动化技术实现B站应用交互...",
    "author": ["Operit Team"],
    "category": "UI_AUTOMATION",
    "env": ["BILIBILI_SESSDATA"],
    "tools": [
        {
            "name": "search_video",
            "description": "在B站搜索视频内容",
            "parameters": [
                {
                    "name": "keyword",
                    "description": "搜索关键词",
                    "type": "string",
                    "required": true
                },
                // ... more parameters
            ]
        },
        // ... more tools
    ]
}
*/
```

-   `name`: 脚本的唯一标识符。
-   `display_name`: （可选，推荐）用于界面显示的名称。不会影响脚本 ID；脚本 ID 仍由 `name` 决定。支持字符串或多语言对象（见 3.1.2）。
-   `description`: 对脚本功能的详细描述。
-   `author`: （可选）作者信息，支持单个字符串或字符串数组。
-   `category`: （可选）脚本分类，用于在工具列表中分组和检索。未填写或为空时，系统会自动归类为 `Other`（见 3.1.3）。
-   `env`: （可选）字符串数组，声明该脚本/包运行时依赖的环境变量名称，例如各类 API Key。应用会根据这里列出的键在“环境配置”界面中展示对应的输入项，并在激活包前校验这些变量是否已经配置。
-   `tools`: 一个数组，定义了该脚本暴露给外部调用的所有工具（函数）。
    -   `name`: 工具的函数名。
    -   `description`: 工具功能的描述。
    -   `parameters`: 工具接受的参数列表，每个参数都应定义 `name`, `description`, `type`, 和 `required`。
    -   `advice`: （可选）标记“仅提示/说明”的工具（如 usage_advice / workflow_guide）。
        -   `advice: true` 时，该工具不要求在脚本中有同名函数实现。
        -   应用于纯提示/规则说明场景，运行时会跳过“工具不存在”的校验。

### 3.1.1. 动态工具集：`states`

当同一个脚本在不同设备能力/权限等级下需要暴露不同工具集（或同名工具需要不同的说明/参数约束）时，可以在 `METADATA` 中使用 `states` 字段。

`states` 为一个数组，每个元素是一个“状态”（State）。系统会根据运行时能力（capabilities）按顺序评估每个 state 的 `condition`，选择第一个为 `true` 的 state 并激活。

#### State 结构

-   `id`: 状态 ID（字符串），用于脚本侧识别当前状态。
-   `condition`: 条件表达式（字符串），使用“类 JS”布尔表达式语法。
-   `inheritTools`: 是否继承顶层 `tools`（布尔值）。
-   `excludeTools`: 需要从继承工具中排除的工具名列表（字符串数组）。
-   `tools`: 该 state 额外提供的工具列表（同 `tools` 的元素结构）。如果与继承工具同名，则覆盖其定义。

#### 选择规则

-   若 `states` 为空：使用顶层 `tools`。
-   若 `states` 非空：从上到下找第一个满足 `condition` 的 state。
    -   找不到任何匹配：回退到顶层 `tools`。

#### 合并规则

-   当 `inheritTools=true`：以顶层 `tools` 为基底。
-   先应用 `excludeTools` 删除指定工具。
-   再将 state 的 `tools` 合并进来（同名覆盖，不同名新增）。

#### Condition 语法（简述）

-   字面量：`true` / `false` / `null`
-   逻辑：`!` / `&&` / `||`
-   比较：`==` / `!=` / `>` / `>=` / `<` / `<=`
-   成员测试：`in`（示例：`android.permission_level in ['ADMIN','ROOT']`）
-   括号：`(...)`
-   数组字面量：`[...]`

#### 可用 capability key（内置）

-   `platform.name`: 当前运行平台名称（如 `windows` / `linux` / `android` / `macos`）
-   `platform.windows`: 当前运行平台是否为 Windows（boolean）
-   `platform.linux`: 当前运行平台是否为 Linux（boolean）
-   `platform.android`: 当前运行平台是否为 Android（boolean）
-   `platform.macos`: 当前运行平台是否为 macOS（boolean）
-   `platform.ios`: 当前运行平台是否为 iOS（boolean）
-   `ui.virtual_display`: 是否具备虚拟屏能力（boolean）
-   `android.permission_level`: 权限等级（enum，会以字符串形式参与比较）
-   `android.shizuku_available`: Shizuku 是否可用（boolean）
-   `ui.shower_display`: Shower 虚拟屏是否可用（boolean）

#### 脚本侧获取当前 state

运行时会向脚本环境提供全局函数 `getState(): string`，返回当前激活的 state 的 `id`。

#### 脚本侧获取当前语言

运行时会向脚本环境提供全局函数 `getLang(): string`，返回当前使用的语言代码（如 `zh` / `en`）。若语言不可用则返回 `en`。

### 3.1.2. 文本字段的双语/多语（LocalizedText）

在 `METADATA` 中，以下文本字段都支持“单语字符串”或“多语对象”两种写法：

-   包级：`display_name`
-   包级：`description`
-   工具级：`tools[].description`
-   参数级：`tools[].parameters[].description`
-   环境变量（若使用对象格式 env 声明）：`env[].description`

#### 两种写法

1) **单语（字符串）**

```json
"description": "一个简短描述"
```

2) **双语/多语（对象）**

```json
"description": {
  "zh": "中文描述",
  "en": "English description",
  "default": "Fallback description"
}
```

#### 语言 Key 的选择与回退

运行时会根据系统语言做选择，优先级大致为：

-   优先匹配完整语言标签（如 `zh-CN`、`en-US`，含大小写变体）
-   再匹配语言代码（如 `zh`、`en`）
-   再回退到 `default`
-   若仍未匹配到，则回退为对象中的任意一个值

#### 示例（包/工具/参数/环境变量同时双语）

```typescript
/*
METADATA
{
  "name": "MyBilingualPackage",
  "category": "Utility",
  "display_name": {
    "zh": "双语示例包",
    "en": "Bilingual Demo Package",
    "default": "Bilingual Demo Package"
  },
  "description": {
    "zh": "演示双语元数据",
    "en": "Bilingual metadata demo",
    "default": "Bilingual metadata demo"
  },
  "env": [
    {
      "name": "MY_API_KEY",
      "description": {
        "zh": "用于访问某 API 的密钥",
        "en": "API key for accessing a service",
        "default": "API key"
      },
      "required": true
    }
  ],
  "tools": [
    {
      "name": "hello",
      "description": {
        "zh": "向指定的人问好",
        "en": "Say hello to someone",
        "default": "Say hello"
      },
      "parameters": [
        {
          "name": "name",
          "description": {
            "zh": "要问好的人名",
            "en": "Name to greet",
            "default": "Name"
          },
          "type": "string",
          "required": true
        }
      ]
    }
  ]
}
*/
```

### 3.1.3. `category` 字段规范

`category` 为 **可选字段**，类型为字符串，用于脚本分类展示与检索。建议直接复用 `examples/packages/` 中已使用的分类，避免创建语义重复的新分类。

```json
"category": "Utility"
```

当 `category` 缺失、为空字符串或仅包含空白字符时，解析阶段会自动归类为：

```json
"category": "Other"
```

本 Skill 的 `examples/packages/` 中可参考的分类值：

-   `Automatic`
-   `Chat`
-   `Development`
-   `Draw`
-   `File`
-   `Life`
-   `Map`
-   `Media`
-   `Memory`
-   `Network`
-   `Search`
-   `System`
-   `Utility`
-   `Workflow`
-   `Other`（系统默认兜底分类）

### 3.3. 使用内置工具 (Tools)

平台提供了一个全局的 `Tools` 对象，它包含了所有与底层系统交互的API。这些API被分类到不同的命名空间下:

-   `Tools.System`: 系统级操作，如 `sleep()`, `startApp()`, `stopApp()`。
-   `Tools.UI`: UI自动化操作，如 `getPageInfo()`, `pressKey()`, `swipe()`, `setText()`。
-   `Tools.Files`: 文件系统操作，如 `read()`, `write()`, `list()`。
-   `Tools.Network`: 网络请求，如 `httpGet()`, `httpPost()`。
-   `UINode`: 用于表示和操作UI元素的类。

所有这些工具函数都是**异步**的，调用时必须使用 `await`。

**示例：**
```typescript
// 等待3秒
await Tools.System.sleep(3000);

// 获取当前页面信息
const pageInfo = await Tools.UI.getPageInfo();

// 在屏幕上滑动
await Tools.UI.swipe(540, 1800, 540, 900);
```

### 3.4. Java/Kotlin 类桥接 (Java Bridge)

Java/Kotlin Bridge 是需要对应 host 实现支持的专用能力，类型声明不代表每个平台都能调用 Java 或 Android 类。全平台插件的文件、网络和终端操作应使用统一 `Tools` 与 host API，不要用 Java Bridge 替代这些抽象。以下示例仅说明已提供 Java Bridge 的宿主上的调用契约。

这套能力适用于：

-   直接访问 Android SDK 类（如 `android.os.Build`、`android.os.SystemClock`）。
-   调用宿主应用暴露的类与单例对象。
-   在脚本中实现 Java 接口回调（如 `Runnable`、`Callable`、Listener）。

#### 3.4.1. 插件入口与 host 边界

插件使用 `Java` / `Kotlin` API；`Kotlin` 是 `Java` 的同义别名。
参数与返回值通过结构化 host 绑定传递，插件不应调用内部 `__operitNativeJava*` 函数，
也不需要编码参数 JSON、解析结果文本或管理对象句柄。
只有 host 明确实现的能力可执行；未实现的操作直接报错。

#### 3.4.2. 高层 API 常用能力

-   类获取：
    -   语法糖（推荐）：`Java.java.lang.StringBuilder`、`Java.android.os.Build.VERSION`、`Java.android.app.AlertDialog.Builder`
    -   兼容写法：`Java.type("java.lang.StringBuilder")`
    -   别名：`Java.use(...)` / `Java.importClass(...)`
-   包链访问：`Java.java.lang.System.currentTimeMillis()`
-   静态调用：`Java.callStatic(className, methodName, ...args)`
-   异步挂起调用：`Java.callSuspend(className, methodName, ...args)`（返回 Promise）
-   构造实例：`Java.newInstance(className, ...args)` 或 `new Java.java.util.ArrayList()`
-   接口实现：
    -   `Java.implement(interfaceNameOrNames, impl)`（支持字符串接口名或 `Java.xxx` 类代理）
    -   `Java.proxy(interfaceNameOrNames, impl)`（`implement` 别名）
    -   **语法糖**：当 Java/Kotlin 方法参数为接口类型时，可直接传 JS 函数/对象，桥接会自动推断接口并创建代理（无需显式 `implement`）
-   生命周期管理：
    -   Java 实例 handle 与 JS 接口回调都由运行时自动管理，无需手动释放脚本侧回调标记

#### 3.4.3. 句柄与生命周期建议

Java 复杂对象跨桥接会被包装成“实例代理”（内部是 handle 句柄），建议：

-   `Java.implement` / `Java.proxy` 产生的回调标记不再需要手动释放；对应 Java 代理被 GC 后，运行时会自动解除当前 JS 回调注册。
-   Java 实例代理的 handle 解绑由运行时自动处理；业务脚本不再暴露 `obj.release()` / `Java.release(...)` / `Java.releaseAll()`。
-   运行时会在代理对象被 GC 后尝试解绑对应 handle，也会在引擎销毁时清理剩余句柄；GC 时机本身仍然是不确定的。
-   Java Bridge 实例默认推荐 `obj.methodName()` 动态语法糖；运行时会优先把实例成员按方法解释，尽量保证这种写法可用。
-   `obj.call('methodName', ...)` 仍可用于极少数字段/方法同名冲突，或在调试桥接问题时做显式调用。
-   `Java.implement(...)` / `Java.proxy(...)` 的 JS 回调会被调度回 QuickJS 运行时线程执行，而不是直接在 Java 调用方线程里执行；不要把它当成 JS 侧真正的多线程执行模型。
-   “Java 返回值会被归一化” 与 “显式构造 Java 对象” 是两件事：例如 Java 方法返回的 `JSONArray` / `JSONObject` / `Map` / `List` 会在 JS 侧按数组或 plain object 使用；但 `new Java.java.lang.StringBuilder()`、`new Java.java.util.ArrayList()` 这种显式构造结果本身仍然是 Java 实例代理。

#### 3.4.4. 示例：包链语法 + 回调

```typescript
const Thread = Java.java.lang.Thread;
const Runnable = Java.java.lang.Runnable;

let runCount = 0;
const runnable = Java.implement(Runnable, () => {
    runCount += 1;
});

const worker = new Thread(runnable);
worker.start();
worker.join(2000);
console.log("runCount=", runCount);
```

#### 3.4.5. 示例：Android 类与内部类

```typescript
const Build = Java.android.os.Build;
const Version = Java.android.os.Build.VERSION;
const AlertDialogBuilder = Java.android.app.AlertDialog.Builder;

console.log("brand=", String(Build.BRAND || ""));
console.log("sdk=", Number(Version.SDK_INT));
```

#### 3.4.6. 结构化参数与结果

高层 API 负责传递结构化参数、传播 host 异常并包装对象句柄。例如，
当前运行时的应用上下文兼容对象可以直接调用已实现的方法：

```typescript
const context = Java.getApplicationContext();
console.log(context.toString());
```

返回字符串保持应用数据本身的含义，不会依据内容解析成错误对象。
静态字段应显式使用 `getStatic` / `setStatic`，嵌套类应使用完整类名；
不会通过捕获字段错误去猜测另一种成员调用方式。

TypeScript 开发引用 `types/index.d.ts` 获取 `Java`、`Kotlin` 的类型提示。

#### 3.4.7. 示例：suspend 调用（callback / Promise）

`callSuspend` 用于调用 Kotlin `suspend` 方法，始终返回 `Promise`。

```typescript
const EnhancedAIService = Java.com.ai.assistance.operit.api.chat.EnhancedAIService;

// Promise 形式
const service = await EnhancedAIService.callSuspend(
    "getAIServiceForFunction",
    ctx,
    FunctionType.CHAT
);
```

## 4. 编写第一个脚本 (TypeScript)

推荐使用 TypeScript 来编写脚本，这样可以充分利用 `types/` 目录中提供的类型定义，获得更好的开发体验。

### 步骤 1: 创建 `.ts` 文件

在你选择的脚本目录下创建一个新的 `.ts` 文件，例如 `my_new_script.ts`。

### 步骤 2: 添加元数据

在文件顶部添加 `METADATA` 块，定义你的脚本和工具。

```typescript
/*
METADATA
{
    "name": "MyNewScript",
    "description": "一个用于演示脚本开发的新脚本。",
    "category": "Utility",
    "tools": [
        {
            "name": "hello_world",
            "description": "向指定的人问好。",
            "parameters": [
                {
                    "name": "name",
                    "description": "要问好的人名",
                    "type": "string",
                    "required": true
                }
            ]
        }
    ]
}
*/
```

### 步骤 3: 编写主体逻辑

导出名称与 `METADATA.tools[].name` 保持一致。直接返回真实工具结果，执行错误由当前运行时记录；不要把失败转换成成功返回。

```typescript
/// <reference path="../../types/index.d.ts" />

/** Greets the named user after a host-backed sleep operation. */
async function hello_world(
    params: { name: string }
): Promise<{ success: boolean; message: string }> {
    await Tools.System.sleep(500);
    return {
        success: true,
        message: `你好, ${params.name}! 欢迎使用脚本。`,
    };
}

exports.hello_world = hello_world;
```

### 步骤 4: 使用类型

-   在文件顶部添加 `/// <reference path="../../types/index.d.ts" />` 可以让 TypeScript 编译器和你的IDE（如 VS Code）找到全局的类型定义。
-   `types/` 目录下的 `.d.ts` 文件详细定义了所有可用工具的签名和返回类型。Java Bridge 的签名以随 Skill 携带的声明为准，调用前核对当前宿主能力。

## 5. 当前版本示例

示例由当前版本软件随 PackageBuilder 提供，位于 Skill 的 `examples/packages/`。下列文件实际来自当前源码；构建产物并不代表它们已安装到当前 runtime。

- `examples/packages/buildin/time.ts`：单文件工具包的 `METADATA`、参数与导出函数。
- `examples/packages/buildin/operit_editor.ts`：实际执行平台编辑命令的包；`operit_editor` 工具将 `args` 字符串数组传给 `Tools.SoftwareSettings.exec(args)`，与 `execute_cli_command` 使用同一执行入口，返回真实结果与错误，不接受自然语言 `query`。
- `examples/packages/external/template_try/`：ToolPkg 清单、工作流模板、工作区模板和资源注册。
- `examples/packages/buildin/workflow/`：较完整的 ToolPkg 项目，包含 UI、公共接口和构建说明。

读取所选示例的实际 manifest、package.json 和源码，再确定项目结构、API 版本及构建命令，不使用旧版仓库中的固定示例路径。

## 6. UI自动化详解

UI 自动化由当前 host 提供；不同宿主暴露的能力有明确范围。不要把 Android 页面结构或应用标识当作全平台通用值；调用前查看实际工具与返回结果。

UI自动化是许多脚本的核心。

### `UINode` 对象

`Tools.UI.getPageInfo()` 返回的页面结构是一个 `UINode` 对象树。每个 `UINode` 代表一个屏幕上的UI元素，你可以通过它来：
-   查找子元素 (`findById`, `findByText`, `findByClass`, `findAllBy...`)
-   获取元素属性 (`text`, `contentDesc`, `bounds`, `resourceId`)
-   执行操作 (`click()`)

### 查找元素

查找元素是自动化的第一步。

```typescript
// 获取当前页面的根节点
const page = await UINode.getCurrentPage();

// 1. 通过资源ID查找 (最稳定)
const searchBox = page.findById('com.example:id/search_box');

// 2. 通过文本内容查找
const loginButton = page.findByText("登录");

// 3. 通过类名查找
const allTextViews = page.findAllByClass('TextView');

// 4. 通过内容描述 (contentDescription) 查找
const backButton = page.findByContentDesc("返回");
```

### 与元素交互

找到元素后，可以对其进行操作。

```typescript
if (loginButton) {
    await loginButton.click();
    await Tools.System.sleep(2000); // 等待页面跳转
}
```

## 7. 调试

- 将不确定的逻辑缩为有 `METADATA` 和导出函数的独立测试包，通过 `package import/enable/list/show/exec` 验证真实行为。
- operit_editor 的 `operit_editor` 工具实际执行 `args` 指定的 core command，`args: []` 读取实际命令帮助。它不提供任意脚本片段直跑功能；不要编造其工具名。
- 查看 `["log", "package"]`、`["log", "show"]` 的实际日志，保存原始失败信息。工具调用失败后停止并定位，不切换另一套执行链路。
- UI、hook、provider 注册需要安装并在应用对应场景测试，不能仅执行一个导出函数就宣称这些能力已经通过测试。
- 构建、安装、启用、执行与发布分别核验，交付时明确说明哪些环节已验证。

## 8. 编译与打包

按第 2 节的项目配置将 TypeScript 输出为 CommonJS JavaScript。单文件包保留元数据与导出函数；ToolPkg 根据 manifest 收集入口、子包、UI、资源和源码。

`Tools.Files.zip(sourceVfsPath, artifactVfsPath, false)` 可以生成不含外层目录的 ToolPkg 归档。成品放在源码目录之外，扩展名为 `.toolpkg`，manifest 位于归档根目录。打包细节见 [ToolPkg 格式指南](./TOOLPKG_FORMAT_GUIDE.md)。

## 9. 安装、更新与测试

使用 [当前版本插件创作流程](./PLUGIN_CREATION_WORKFLOW.md) 中的首次安装与同 ID 更新步骤。Core command 参数数组不含 `operit2`；脚本可调用 `Tools.SoftwareSettings.exec(args)`，AI 可直接调用系统工具 `execute_cli_command`。

`package import` 接收成品文件，不接收源码目录，并拒绝重复的已注册包名。更新前完成构建、记录配置并获得删除授权，再明确执行删除、导入、恢复配置与重新测试。不能把源码同步、命令回显或一次工具调用完成解释为热更新成功。

## 10. 编辑器辅助

可使用编辑器的 TypeScript 提示和项目构建任务。编辑器负责本地源码与构建检查；真实插件行为必须在当前 Operit runtime 内测试。仓库没有通用的 ADB 脚本执行或一键烧录流程，不配置指向不存在工具的 VS Code 任务。
