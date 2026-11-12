# 模型目录与模型识别

拉取模型、添加模型和对话时用到的价格、上下文长度、能力（工具调用、图片/音频/视频输入）
和思考档位来自 [models.dev](https://models.dev) 目录。程序内置一份完整快照，首次安装离线也能
进入模型界面并识别能力；后台下载成功后，完整替换当前使用的数据。

## 数据来源与更新

- Core 下载 `https://models.dev/api.json`（各服务商的模型）和 `https://models.dev/models.json`
  （各厂商的模型，补充没有服务商列出、但账号可以调用的模型），保存在设备本地的
  `runtime/cache/models_dev/`，不参与 Space 同步。实现见
  `core/crates/runtime/application/src/services/ModelCatalogService.rs`。
- `tools/build_scripts/models_dev_bundle.py` 下载两份目录并校验后，原子更新
  `core/crates/runtime/application/assets/models_dev/catalog.json.gz`。App 和 CLI 的发布构建通过
  公共 Cargo 构建入口自动执行；Web 发布流水线也会刷新。该快照由 `.gitignore` 排除，不提交
  上游目录数据。开发构建入口在快照缺失时自动准备，已有快照的开发检查不要求联网。
  首次直接使用 Cargo 检查或直接打发布包时，先手动运行该 Python 脚本。
- 内置目录是初始默认数据，不作为最终权威。设备上已完成下载的目录优先使用，下载成功后整份
  替换当前目录，重启也继续读取下载版本。内置版本不会写成已下载的缓存，也不会推迟首次更新。
- 启动时在后台检查，运行期间每小时检查一次，已下载目录超过 24 小时会更新。进入模型界面、
  读取配置、添加或列出模型时直接使用当前目录，不等待 models.dev 请求。
- 下载失败保留当前目录，只记诊断日志和 `getModelCatalogStatus.lastError`，不作为模型配置命令
  错误上抛，界面也不显示后台下载失败。30 分钟内不会自动重试；界面「更新」按钮可手动重试
  （`refreshModelCatalog`）。两份目录完整下载、解析和保存成功才替换，失败不会采用半份目录。
- 本地运行的模型不依赖此目录。模型服务商自身的模型列表请求和连接错误仍按原流程处理。
- 没有模型列表接口的服务商（ChatGPT Codex）使用 `core/crates/foundation/model/collects/ModelCatalog.rs`
  里手工维护的固定列表。

## 识别规则

匹配在 `core/crates/foundation/model/src/ModelsDevCatalog.rs`，按下面顺序查找，先找到的生效：

1. 服务商自己在 models.dev 里的条目。服务商按接口地址对应到 models.dev 的 `api` 字段；
   models.dev 没有记录地址的官方服务商（OpenAI、Anthropic、Google 等）按服务商类型对应，但只限
   接口地址在该厂商自己的服务器上。填了中转或自建地址的服务商没有对应的列表：模型列表只列出
   这个地址 `/models` 返回的模型，模型按第 2 条起的规则识别，也不带厂商的价格。
2. 不分服务商，按去掉 `/` 前缀的小写模型 ID 查找。上下文、能力取多数条目的说法，所以同一个
   模型在任何服务商、中转都能识别。条目的 `canonical_model_id` 也算这个模型的 ID。
3. 去掉 Bedrock 区域前缀、`(档位)` 后缀或 `:标签`。
4. 去掉中转的窗口标记（`[1m]`，窗口按标记计）、`claude-` 前缀或渠道词（`nyro-`）。只有剩下的部分
   是已知且带版本号的模型时才去掉。
5. 版本号的点和横线视为相同，忽略大小写、空格和快照日期（`-20260801`、方舟的 `-261024`）。
6. 去掉括号里的渠道标签（`(WB)`、`【WB】`）。

`-pro-max` 和 `-pro`、`-mini`、`:batch`、`-2507` 等变体不会被当成同一个模型。价格只取对应服务商
自己的条目，不使用其他服务商的价格。

## 用户设置与目录的关系

- 模型配置里的 `*Override` 字段只保存用户改过的值，按字段区分。没改过的字段在读取时按目录计算，
  所以目录更新后，已添加模型里用户没改过的字段会跟着更新。
- 保存时，和目录值相同的字段不会记为用户设置。上下文按整千 token 比较，因为设置页按整千显示。
- 目录识别不了的模型，使用添加时服务商列表自带的信息（目前只有 OpenRouter 提供），再使用默认值。
- 本机安装的本地模型不参与识别，按本地运行配置描述。

## 思考档位

- 模型能怎么思考取自 models.dev 条目的 `reasoning` 和 `reasoning_options`：一个开关、若干 effort
  档位、一个 token 预算，或者都没有（总是思考，不能调）。这些按服务商（托管方）记录，所以先取
  服务商自己的条目，再取多数条目的说法；厂商目录（`models.json`）不记录思考控制。
- 档位怎么写进请求由接口协议决定，写在 `core/crates/provider/services/src/chat/llmprovider/ThinkingDialect.rs`：
  OpenAI Chat 的 `reasoning_effort`、Responses 的 `reasoning.effort`、Anthropic 的预算或
  adaptive 加 `output_config.effort`、Gemini 的 `thinkingLevel`/`thinkingBudget`、DeepSeek 等的
  `thinking.type`、通义和硅基流动的 `enable_thinking`/`thinking_budget`、OpenRouter 的 `reasoning`。
  协议写不出的控制不提供档位；一个模型同时有 effort 和预算时，Anthropic 和通义用预算，其他用 effort。
- 档位统一成 `off`、`enabled`、`minimal`、`low`、`medium`、`high`、`xhigh`、`max`，界面在各种语言下都
  显示这些写法。预算档位按 2k/8k/16k/32k token 换算并限制在目录给的范围内，范围上限超过 32k 时
  再加一个 `max`。
- 档位按模型保存（`ModelProfile.thinkingLevel`）。没设置过的模型沿用最近一次在任意模型上设置的
  档位，换成这个模型最接近的一档：先保持开或关，再取强度最接近的，一样近取更强的；从没设置过
  时为 `off`。存的档位后来不再提供时按同样规则换算。
- 目录没有或说得不对的模型，用户可以自己描述（`ModelProfile.thinkingOverride`），描述整体替换
  目录的说法。本机运行的本地模型由内置运行时处理，每个模型都可开关思考。

## 模型连接检测结果

模型设置中的单模型检测和批量检测会在每个模型完成时，由 Rust Core 保存最后一次结果。
保存内容包括整体通过状态、聊天／工具调用／图片／音频／视频各项结果，以及未能启动检测的错误。
关闭供应商弹窗或重启应用后，列表会重新读取结果，恢复通过、失败、受限状态和错误详情。排队和检测中的状态
只存在于当前页面；停止批量检测时，未开始的模型重新显示上一次已完成的结果。

批量检测最多同时测试 32 个模型，待测模型不足 32 个时按实际数量执行。每个模型内部的聊天、
工具调用和媒体检查依次执行；完成一个模型后，立即开始下一个排队的模型。

单模型和批量检测共用独立的输出预算：每个探测请求设置 1,024 token 上限，替换模型配置中的输出
长度参数，采样等其他参数仍沿用模型设置。该预算只用于检测，不写回模型配置。各协议使用其对应
的输出长度字段；ChatGPT Codex 接口不接受这些字段，由已有适配器移除。

HTTP 402（额度或请求预算不足）和 HTTP 429（限流或上游冷却）显示为黄色的「检测受限」，批量
统计单独计数；原始错误仍可查看和复制。受限项不用于关闭模型能力开关，通过的检查和其他已完成
检查仍正常应用。已保存的旧结果也在读取时按相同规则显示。

结果通过 `CoreNodeStateStore` 原子写入
`runtime/diagnostics/model_connection_tests.preferences.json`，归当前 CoreNode 所有，不参与 Space
同步。使用远程 Core 的客户端读取该 Core 的记录，因此结果对应实际发起请求的节点。该文件属于
持久化诊断状态，不放入临时缓存目录。

记录按供应商 ID 和模型 ID 区分，每个模型只保存最新一次结果。连接设置的 SHA-256 摘要用于检查
结果是否仍适用：供应商类型、接口地址、密钥及密钥池启用情况、自定义请求头、限流、模型请求参数
和本地推理配置变化后，旧结果不再显示。摘要不保存密钥原文，密钥使用次数和轮换位置不影响结果。
检测后自动应用的能力开关、显示名称、价格和摘要设置不使刚完成的检测失效。删除供应商或模型时
一并删除对应记录。此前仅留在弹窗内存里的结果需要重新检测一次才能保存。

## 命令行

```text
operit2 model catalog-status
operit2 model catalog-refresh
operit2 model thinking <provider-id> <model-id> [off|enabled|minimal|low|medium|high|xhigh|max]
```

`provider-model-available-list` 的输出会列出每个模型识别成的目录模型。
