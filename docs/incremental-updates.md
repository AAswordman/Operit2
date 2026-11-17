# Release / Preview 跨平台增量更新

源码和开发者明确发布的完整包使用
[`AAswordman/Operit2`](https://github.com/AAswordman/Operit2)。
增量补丁和更新清单发布到独立的
[`AAswordman/Operit2-nightly`](https://github.com/AAswordman/Operit2-nightly)，该仓库不接受完整安装包。
客户端同时检查两个仓库，按版本合并同一平台格式的完整包与补丁。
仓库名不代表新增 nightly 通道，仍只有 release 和 preview。
完整包和补丁可以独立缺席：支持只发布完整包，也支持只发布清单和补丁。
完全没有公开更新元数据的内测，仍由开发者直接分发安装包并使用系统安装器安装。

## 渠道和版本

- 正式版检查两个仓库的 stable release。
- preview/rc/dev 继续使用现有 preview 渠道，检查两个仓库的 stable 和 prerelease。
- 客户端按 SemVer 检查新版本；preview 可以更新到后续正式版。
- 发布器从现有产品版本推导 GitHub prerelease 标记，不增加渠道开关或改写版本。

Windows MSI 保持单调版本映射：`major.minor.(patch * 1000 + revision)`，
dev、preview、rc 的 revision 起点依次为 0、200、400，
各渠道计数范围为 1..199，stable 为 999。计数用完后增加 patch。

Flutter 的平台 build number 仍独立于 SemVer，继续使用现有构建工具管理。
后续安装的完整包必须遵守各平台的 build number 要求，尤其是 Android 的
versionCode 不得降低。增量工具不修改 SemVer 或平台 build number。

## 首次可用版本

新更新器首次随 `2.0.0-preview.36` 分发，从 `.36`
开始可用。此前分发、未包含本实现的 `.35` 仍需通过完整安装包升级到这一版。

本轮 Windows MSI 测试使用重新构建的 Preview.36（构建号 43）→ Preview.37（构建号 44），
包含独立补丁仓库发现、完整退出进程以及 Windows Installer 路径修复。
按开发者要求替换此前的测试包；测试时必须使用本轮分发的原始 Preview.36 MSI。
完整包只在本地分发，独立仓库仅公开补丁和清单。

- Android 单 APK 可直接读取已安装版本的 APK，使用准确的原始 APK 制作对应补丁。
- Windows、macOS、Linux 桌面 App 在点击新版更新后，如需旧包会询问已安装版本的原始
  安装包。选中且准确校验通过后，即可使用增量，无需先完整下载一次。
  CLI 使用保留的原始归档；没有准确基包时，该次更新使用完整包。

这些版本号描述最早发布顺序，实际使用还取决于同平台、架构和发行格式的准确基包、
服务器发布的补丁，以及补丁体积是否小于完整包。正式版使用相同机制。

## 补丁协议

`tools/updates/patch.py` 使用内容分块、SHA-256 匹配和 gzip 压缩的 COPY/ADD 指令。
插入或删除字节后可重新定位后续相同块。协议 `O2PATCH1\0` 包含 base/target 的
字节长度和 SHA-256；指令为 END=0、COPY=1（offset、length）、ADD=2（length、bytes），
所有整数都是 little-endian u64。生成器和客户端均流式读写大文件。

每个完整安装包对应：

```text
<package>.update.json
<package>.from-<完整 base SHA-256>.patch.gz
```

清单 schema 为 1，声明目标版本、准确文件名、长度、SHA-256，以及各个基包的版本、
长度、SHA-256、补丁名、补丁长度和 SHA-256。一个目标最多声明 64 个基包。
同一旧包可以直接生成到当前目标的补丁，无需先安装中间版本。

客户端通过 GitHub API 返回的 digest 验证清单，核对清单的目标摘要与 GitHub 完整包
摘要（发布了完整包时），再验证补丁、基包和重建结果。只发补丁时，目标摘要由经过
GitHub digest 验证的清单提供，仍完整验证重建结果。所有下载 URL 都限于上述两个仓库对应 release
的 HTTPS 资产。平台、ABI、macOS sandbox/unsandbox 各自独立。Windows App 仅分发 MSI。

补丁重建的是原发布安装包的原始字节，包括 APK 签名块、MSI 内容、macOS PKG
签名与其他元数据。重建后进入既有 `SystemOperationHost::installApp`，没有热替换
Dart/Rust 运行代码的新通道。

## 平台基包和安装行为

| 目标 | 基包来源 | 安装 |
| --- | --- | --- |
| Android 单 APK | Android Host 读取当前 `applicationInfo.sourceDir` | 既有 APK 安装器和授权流程 |
| Android split APK | 不声明单包基包能力 | 完整 APK 下载后走系统安装器 |
| Windows MSI | 已校验、保留的 MSI | 既有产品族校验和 MSI 升级助手 |
| macOS unsandbox / sandbox | 已校验、保留的对应 PKG | 既有系统 Installer，保留系统确认 |
| Linux App | 已校验、保留的 tar.gz | 既有等待退出的安装助手 |
| 桌面 CLI/TUI | 已校验、保留的对应归档 | 既有 CLI 安装流程 |

iOS 不具备任意自安装能力，不启用应用内增量安装。模块 ZIP/HAP 等文件可通过工具
生成字节补丁，但这不等于已经增加其系统级应用内安装能力。

Windows 安装后的目录或 Windows Installer 缓存、macOS 已安装的 App 都不能可靠还原
原始签名 MSI/PKG，因此不重新打包安装目录作为基准。

应用不常驻显示“从安装包更新”。发现新版后，侧栏或关于页的 Operit Logo、关于页更新按钮
进入同一个弹窗。用户点击更新时，Core 统一获取并验证更新清单，计算完整包、当前版本
适用且实际已发布的增量补丁、补丁大小，以及系统/本地基包是否可用，保存一个执行方案。
Flutter 不独立推断全量或增量，也不处理安装包内容。

- 方案为完整包：直接下载。
- 方案为增量且已有准确基包：直接下载补丁，合并并校验。
- 方案为增量但缺少基包：询问“你还有上一次安装（版本 xxx）的安装包吗？”，
  操作只有“选择”和“我没安装包了”。
- “选择”：通过既有 Host 文件选择器流式读取，按该方案补丁声明的 base SHA-256 和大小
  验证。改过名称但字节正确的原始包也可使用；文件名不作为身份依据。验证通过后保留基包，
  再下载增量并合并；验证失败说明不符合要求，允许重新选择。取消选择回到询问，不发起下载。
- “我没安装包了”：Core 对同一个方案选择完整包；如果没有发布完整包，提示暂时无法完成更新，
  需自行下载安装包或找到旧安装包，弹窗只有“知道了”。

本次校验通过的旧包写入 `runtime/application_update_bases/<包系列及格式>/`。
下载或重建完成后，安装包保存在本机基包目录，每种发行格式只保留一份，供安装重试和下次增量使用。
临时下载、补丁和合并目录在成功、失败或取消后删除，异常退出残留在下次启动清理。
新版启动删除旧版本基包和已到期的失败记录；已完成安装助手的脚本、日志和结果一并清理。
尚未安装的新版包继续保留，避免删掉系统安装器正在读取的文件。
Android 更新直接共享这份 APK，不再复制到通用打开文件缓存；安装完成后使用系统 APK 作为基包。
用户选择的原始安装包和开发者制作补丁用的历史包不属于客户端清理范围。

## 传输优先级与失败处理

基包匹配和大小比较在补丁传输前完成，完整包和增量是否存在分别由 GitHub 资产列表及
经过认证的清单决定。只有与当前版本、平台和格式相符，且大小严格小于完整包的已发布补丁
才算可用增量。

| 条件 | 本次行为 |
| --- | --- |
| 该目标版本曾提取基包或重建校验失败 | 只允许完整包；缺失则提示暂时无法更新 |
| 有准确可读的基包，且有适用的小体积补丁 | 增量补丁 |
| 有适用补丁但缺少原始基包 | 弹窗询问；根据用户选择验证基包或选择完整包 |
| 没有适用补丁，或补丁大于等于完整包 | 有完整包则直接下载；否则提示暂时无法更新 |
| 可选清单/所选补丁已删除（HTTP 404/410） | 有完整包则使用完整包；否则明确提示无法完成 |

当前协议是一个旧包直接重建到目标版，每次只下载对应基包的一份补丁。
清单中的其他补丁是不同基包的可选路线，不计入本次传输总量；没有补丁链累计下载。
如果后续扩展到补丁链，大小比较必须使用所选整条路线实际需要下载的总字节数。

提取基包、读取或校验基包失败，以及已完整下载补丁的摘要、格式、合并或目标校验
失败时，本次报错并清理本次未提交文件，持久记录该目标版本；下次发起更新直接用完整包。
该决定保存在 `runtime/application_update_state/<包系列及格式>.json`，归属 CoreNode，
重启后生效，不参与 Space 同步，也不受下载任务或基包清理影响。记录仅影响同一目标
版本和包系列，新目标版本、其他架构和安装格式会重新评估增量。

普通 HTTP 中断和用户取消不标记补丁不可用，下次仍可尝试增量。取消会终止 HTTP 和
本地处理。本次增量失败不会自动开启第二次完整下载；完整下载始终继续校验 GitHub
目标摘要和安装包格式，失败记录不会跳过完整包验证。
已下载包的凭证、安装重试、权限确认和重启交互继续使用既有 Core/Flutter 协议。

“文件不存在”和“文件损坏”分别处理：可选文件的 404/410 当次改用完整包；500、超时、
鉴权失败和摘要不一致不会被误认成文件缺失。完整包缺失会明确报错。
停止制作增量后可以一直只发布完整安装包，发布工具不强制要求清单。

只发清单和补丁的 release 也会被自动发现，其目标完整包名由当前平台格式和版本确定，
不会假定完整包可下载。完全没有公开 Release/清单时，客户端无法发现一个从未公布的新版本；
开发者直接分发安装包，由用户使用系统安装器安装。桌面用户保留这份原始包，未来更新时再按需选择。

## 本地制作和发布

以下命令从源码仓库根目录运行；Windows 使用项目 `.venv`。
发布操作使用 `gh` 当前登录状态，不自动读取旧的 `github.env`。

```powershell
gh auth login

# 使用既有平台构建工具生成 tools/release/dist 中的完整包。
# 例如 Windows（本次机制实现不执行此构建）：
.\.venv\Scripts\python.exe tools/build_scripts/build_flutter_windows.py --enforce-lockfile

# 内测发包前保留一份准确原始包，不上传 GitHub，不需要 Release。
.\.venv\Scripts\python.exe tools/updates/archive.py

# 首版没有 base，生成空补丁清单；后续自动使用本地历史并保留本次完整包。
.\.venv\Scripts\python.exe tools/updates/prepare.py

# 已经私下发过的历史包，也可以现在补入历史（使用当时的准确文件及原始文件名）。
.\.venv\Scripts\python.exe tools/updates/archive.py --dist D:/private/preview34 --version 2.0.0-preview.34

# 后续发布可显式提供多个旧版本完整包目录。
.\.venv\Scripts\python.exe tools/updates/prepare.py --base tools/release/work/old-release --base tools/release/work/old-preview

# 完成补丁文件、摘要及 gh 权限检查，不上传。
.\.venv\Scripts\python.exe tools/release/gh_publish.py --patch-only --check-only

# 只公开清单和补丁，完整包仍保留在本地；需要已有实际可用的补丁。
.\.venv\Scripts\python.exe tools/release/gh_publish.py --patch-only --draft

# 上传补丁和清单，核对 GitHub digest 后公开；完整包留在本地。
.\.venv\Scripts\python.exe tools/release/gh_publish.py --patch-only

# 仅在开发者明确要求公开完整安装包时使用。
.\.venv\Scripts\python.exe tools/release/gh_publish.py --full
```

每次制作使用新的 staging 文件集。`prepare.py` 对每个生成的补丁执行独立重建，
只将重建成功且小于完整包的补丁写入清单。工具拒绝覆盖已有补丁。
`--base` 可以重复；同一 SHA-256 只生成一次。

本地历史默认在 `tools/release/bases/<version>/`，与会被构建脚本清理的 `work/` 分离，
已加入 gitignore。`packages.json` 记录每个完整包的大小及 SHA-256，读取历史时逐个验证。
同版本快照不可被不同字节或不同包集合覆盖；应先收集该版全部计划分发的包再归档，
追加/重建分发内容需要增加版本。该目录独立备份，不依赖公开 Release；`--history` 可指定
其他持久目录。`archive.py --root` 指定相同历史根目录。
制作补丁默认使用本地最近三个正式版和三个 preview，按 SemVer 排序；旧快照继续保留供
显式 `--base` 使用。每个目标最多使用 64 个不同基包，超出时工具明确拒绝。

`incremental.py bases --count 3` 使用 `gh` 分别下载最近三个正式版和三个 preview
的完整包，只选择比当前目标旧的版本，并根据 GitHub digest 验证，保存在
`tools/release/work/incremental-bases/`。两个渠道各自保留历史，按 SemVer 排序。
使用已下载的实际子目录作为 `prepare.py --base` 参数；不需要改渠道参数。
该历史集合也支持从 preview 直接更新到正式版。首版没有历史时返回空集合。

也可独立验证协议：

```powershell
.\.venv\Scripts\python.exe tools/updates/patch.py build old.apk new.apk delta.patch.gz
.\.venv\Scripts\python.exe tools/updates/patch.py apply old.apk delta.patch.gz rebuilt.apk
```

## GitHub Actions

Windows Release Build、Linux Release Build、Android Flutter Build 和 macOS Flutter Build
继续使用源码中已有的产品版本与平台 build number，无新增版本输入。

```powershell
gh workflow run "Windows Release Build" -f products=all
gh workflow run "Linux Release Build" -f products=all
gh workflow run "Android Flutter Build"
gh workflow run "macOS Flutter Build" -f products=all
```

构建成功后，在相同源码 commit 的 checkout 中收集和发布：

```powershell
.\.venv\Scripts\python.exe tools/release/incremental.py publish --run-id <windows-run> --run-id <android-run> --run-id <macos-run> --draft
```

该入口核对所有 run 已成功且 `headSha` 等于本地 HEAD，要求空 dist，然后收集当前版本
的完整包，使用本地历史及两个渠道的在线历史基包，生成并验证补丁，保留本次完整包，创建草稿。
`--local-bases-only` 只使用本地历史，不读取历史 Releases；`--full-only` 跳过增量制作及
历史 Release 下载，仅发布并归档完整包；`--patch-only` 生成并保留完整包后只上传非空清单和补丁，
没有实际补丁则拒绝发布。`--full-only` 与 `--patch-only` 互斥。
默认分别发布到两个仓库：完整包进入主仓库，非空清单和补丁进入独立补丁仓库。
`--patch-only` 始终选择独立补丁仓库；`--full-only` 只选择主仓库。
发布器拒绝交叉目的地，完整包发布模式也会排除所有增量文件。完整包只在开发者明确授权时发布。

`Publish Release and Preview` workflow 提供相同入口，输入准确的 40 字符源码 SHA
和逗号分隔的 run IDs。默认只创建草稿，可取消 draft 直接发布。主仓库发布使用
`github.token`，权限为 Actions 读取及 Contents 写入。
签名密钥继续使用既有各平台构建工作流的配置。完整包的 Tag 指向构建使用的准确源码提交，
补丁仓库的发布说明链接到该源码提交。
工作流的 `full_only` 选项只发布完整包，`patch_only` 只发布补丁及非空清单；两者互斥。
包含增量的发布需要配置 `OPERIT_PATCH_TOKEN`，允许读取源码仓库构建并写入这两个仓库。

发布器先上传隐藏草稿，检查上传后的每个 GitHub digest 和完整资产集合，再公开 release。
公开后禁止覆盖资产；重建发布必须增加产品版本，保证客户端基包身份稳定。
同一草稿可以重试上传，残留多余资产会明确报错并保留草稿供检查。

## 验证

```powershell
.\.venv\Scripts\python.exe -m unittest discover -s tools/tests -p test_incremental_updates.py -v
.\.venv\Scripts\python.exe -m unittest discover -s tools/tests -p test_update_base_archive.py -v
cargo test --manifest-path core/Cargo.toml -p operit-util --test incremental_updates
```

Python 测试覆盖内容偏移、跨容器格式、生成确定性、越界、损坏、截断、清理和发布清单。
Rust 集成测试消费 Python 生成的真实 fixture，经 Host/VFS 重建并比较目标完整字节。
这些检查不能代替真实签名安装包、设备安装、macOS 系统确认和 Windows 退出后升级的验证。
