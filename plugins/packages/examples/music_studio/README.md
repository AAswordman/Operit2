# Operit Music Studio / 音乐工作台

AI 优先的 external 插件。TypeScript + 自包含 HTML/WebView；音频由固定内存 C/WebAssembly DSP 合成，Web Audio AudioWorklet 负责实时输出，不包含录音采样、SoundFont、在线生成服务或运行时 CDN。

## 默认试听：ECLIPSE · 逐光

原创 D minor / 150 BPM / 4⁄4 melodic dubstep 展示工程：48 小节、19 轨、约 1700 个音符事件，音乐 76.8 秒。包含氛围 Intro、渐进 Trance Build、两个不同的 Drop、Breakdown、二次蓄力和 Outro。详见 [曲目设计](docs/ECLIPSE.md)。

首次打开自动创建；**已有工程不会被示例覆盖**。已有用户从「音色库 → 工程模板 → ECLIPSE」创建一份新工程。默认全曲 FIT 总览；缩放、段落跳转和钢琴卷帘用于检查局部。

## 安装与开发

构建需要 LLVM 的 `clang` 和 `wasm-ld`，两者必须位于 PATH 中。DSP 构建脚本统一通过 PATH 调用工具。Windows 安装 LLVM 后，将其 `bin` 目录加入用户 PATH，并重新打开终端。例如本机安装目录为 `C:\Program Files\LLVM\bin`：

```powershell
$env:Path = "C:\Program Files\LLVM\bin;$env:Path"
clang --version
wasm-ld --version
```

`sync_plugin_packages.py` 会在打包前使用 Corepack/pnpm 安装锁文件指定的依赖。手动开发时，在此目录执行：

```sh
npm ci
npm run pack:toolpkg
# 导入 dist/music_studio.toolpkg 并启用插件
npm run preview
# 在终端给出的 localhost 地址预览
```

宿主入口：侧边栏插件 / 工具箱中的「音乐工作台」。首次点击播放解锁音频。网页预览保存到浏览器 localStorage；宿主模式保存到 PluginConfig，由主运行时统一管理。两者数据不混用。

```sh
npm run typecheck
npm test             # DSP 测试使用 resources/dsp/studio.wasm；改 C 后先 npm run build:dsp
npm run test:browser  # 需要本机 Google Chrome；先 npm run build
```

若本机音频驱动无法初始化 AudioWorklet，可显式使用 `MUSIC_STUDIO_SKIP_LIVE=1 npm run test:browser` 仅跳过实时播放检查；这不能视为实时播放验收通过。

浏览器测试检查六种视口、对象入口/右侧检查器/乐段编辑页切换与范围保护、播放不中断、段落 seek、循环边界、真正 L/R 电平，以及完整离线 WAV 的声部数、峰值、立体声和段落能量。测试生成 `.test-output/eclipse.wav` 与分析 JSON（不打包、不入库）。这些是自动化检查，不代表 Android/iOS 真机性能或主观听感已验收。

## 功能

- 5 种合成引擎：双振荡器/谐波波形、FM、合成乐器层、程序鼓机、立体声环境噪声；32 个原创预置。
- 7 类效果器：三段 EQ、低通、过载、立体声 Chorus、节拍/乒乓 Delay、程序生成立体声混响、压缩。
- 双振荡器 A/B 独立波形、1–8 声部、失谐与立体声宽度；B 八度/半音/音分调音、固定/随机起始相位。
- 居中正弦 Sub（0/−1/−2 八度）、噪声层、ADSR、实际共振低通、滤波包络/八度 LFO、音高滑动。
- Haas 单侧延迟（−35..35 ms）与干湿比；效果器后 M/S 低频收窄（0..500 Hz，12 dB/oct 侧信号高通）。轨道音量、声像、静音/独奏。
- 新增「Reese / 厚低音叠层」「Haas / 宽中频低音」预置；原有工程自动补齐新参数，默认关闭 Sub、噪声、Haas、低频收窄，不覆盖用户音符。
- 每轨 level / pan / cutoff 自动化；示例把底鼓位置编译成音量避让包络，**不是通用外部 sidechain 输入检测器**。
- 时间线、钢琴卷帘、鼓步进、总线限制器、实际双声道电平与频谱、节拍器、工程 JSON 和 PCM16 立体声 WAV 导出。
- 7 个 AI 子包、32 个工具，支持原子编曲事务、版本冲突检查、撤销/重做；详见 [AI 接口](docs/AI_TOOLS.md)。

默认只显示编排时间线；**底部只保留播放、速度与节拍器**，不再常驻五个编辑器标签。

### 对象驱动的编辑布局

- 点击轨道名称下的乐器按钮：宽屏打开右侧设备检查器（乐器／效果链），平板与手机进入独立设备页。轨道的 FX 按钮直接打开效果链。
- 点击时间线的乐段块：打开该乐段的完整钢琴卷帘工作页，标题显示轨道与小节范围；手机默认显示 1 小节，可切换 1/2/4/8 小节缩放。鼓乐段可在钢琴卷帘与鼓步进之间切换。
- 返回／关闭／Escape 恢复编排和原滚动位置；选择设备或乐段不会 seek 或暂停播放。
- 乐段由当前音符、乐曲段落和空白小节自动分组，**不是新增的持久 Clip 数据模型**，没有实现独立 Clip 拖动、复制或拉伸。旧工程无需转换。
- 双击轨道空白或空轨道的「新建乐段」打开一个空白编辑窗口，添加音符后形成乐段块；未写入音符的空窗口不会保存为持久空 Clip。
- 钢琴卷帘、鼓步进及「填充当前窗口」仅修改所选范围内的音符；生成操作可撤销，不再替换全轨。
- 顶栏提供音色库、总控、AI；工程设置与导入／导出在工程菜单。手机侧面板铺满工作区，不再从底部层层堆叠。

只有时间线或当前面板内部滚动，不堆叠多个整页栏目。

### 合成器调音

「合成器」页直接显示 OSC A/B 波形与叠层控制，不再把关键振荡器参数藏在折叠面板内。A 使用 `unison/detune/width`，B 使用 `unisonB/detuneB/widthB`。`blend` 为 B 占比；Sub、噪声独立于此混合。FM 仍是两算子，不套用双振荡器的声部参数。

Haas 延迟正值延迟右声道，负值延迟左声道；`haasMs=0` 或 `haasMix=0` 旁通。低频收窄位于插入效果器和 Haas 后、轨道声像前，衰减低频侧信号，**不是砖墙式完全单声道**。Haas 的单声道折叠可能产生音色变化，需要对照试听。声部数越高 CPU 开销越大，同时音符上限仍为 64。

### 环境纹理引擎

`atmosphere` 用三段宽共振带通塑造气流，两个慢速调制连续漂移；不是简单白噪声加正弦波，也不是实地录音。

- `brightness` 同时改变共振中心和高频占比；`noiseLevel` 控制气流量，0 可关闭气流。
- `blend` 在环境引擎中为谐波比例，谐波仅使用当前音高的整数泛音并进行奈奎斯特渐隐；不套用 OSC B 混合语义。
- `width` 控制独立 mid/side 气流的宽度，0 为居中，展开时补偿噪声增益；不依赖 Haas 延迟。后续立体声效果器仍可重新扩宽声音。
- 每个声部使用独立随机状态，分频系数按声部采样时钟插值；明亮度、气流量、谐波量和宽度实时平滑。
- 旧工程已保存的环境参数不被自动覆盖；旧参数 `noiseLevel=0` 时可从 AIR 面板开气流，或重新选择 Aether 预置。

### DSP 音质与叠层

- 双振荡器各支持 1–8 声部。三角波使用 8 个奇次泛音；三角／Glass／Hollow 的泛音在接近奈奎斯特频率时渐隐，超限泛音不再折回。锯齿／方波继续使用 PolyBLEP，未实现整个效果链的过采样。
- 失谐采用对称、中心加权分布；零失谐且零随机相位的同相叠层按声部数补偿增益，其他叠层按平方根补偿。更多声部并不等于更好的音色，也会增加 CPU 开销。
- 合成声部的滤波包络／LFO 按音符采样时钟更新，每 16 个采样点计算目标系数并插值，输出不再依赖调用方的音频块尺寸。起音使用平滑曲线，释音使用二次衰减曲线。
- 以上是自主减法／谐波合成器的质量改进，不是商业波表合成器复刻；效果器过采样、复杂波表变形和真实乐器采样仍未实现。音色是否合适须实际试听，测试不会替代主观听感。

## 工程布局

```text
manifest.json          ToolPkg 注册、子包和资源声明
src/main.ts            插件入口、IPC 与导航
src/api.ts             公共 API
src/service.ts         唯一持久状态、事务、命令回执
src/host-service.ts    PluginConfig 适配
src/packages/          7 组 AI 工具和 IPC client
src/shared/            模型、校验、音色、自动化、编曲与模板
src/ui/screen.ts        宿主 WebView 桥接
web/audio/             实时/离线共用合成器与效果器
web/ui/                DAW 布局与 Canvas 绘制
web/app.ts              交互、同步、播放和导出
scripts/               构建、打包、预览
resources/             自包含 HTML 与声明
dist/                 编译 JS 和 .toolpkg（归档不入库）
tests/                 模型、服务、音频和响应式测试
docs/                  AI 接口和示例编曲说明
```

## 边界与性能

- 24 轨、每轨最多 6000 音符、每轨 6 效果、64 个并发音符声部（unison 内部振荡器另计）、128 小节、30 个工程。
- WAV 音乐部分最多 90 秒，额外固定 3.2 秒尾音；长反馈延迟/混响可能超过尾音窗口，不自动无限延长。44.1 kHz / 双声道 / PCM16。
- 实时由 AudioWorklet 按块调度并调用 WASM；进入后台暂停，关闭 WebView 后不继续播放。宿主需支持安全上下文中的 AudioWorklet 和 WebAssembly。
- 离线渲染在 Worker 中复用同一 WASM DSP 与调度器，不创建每音符音频节点；耗时取决于设备。不是实时录音。
- DSP 使用固定 48 MiB WASM 内存，包含合成器及七类效果器；JS 负责界面与调度。未实现 VST/AU 宿主、Kontakt 采样兼容、商业 Serum 音色加载、音频录音、MIDI 硬件或母带响度认证。
- 音色是自主简化设计，不是商业合成器的完整复刻。无第三方旋律、预置或音频素材。
