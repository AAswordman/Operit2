# 模型目录与模型识别

拉取模型、添加模型和对话时用到的价格、上下文长度、能力（工具调用、图片/音频/视频输入）
和思考档位来自运行时下载的 [models.dev](https://models.dev) 目录。程序里不再编译模型数据快照。

## 数据来源与更新

- Core 下载 `https://models.dev/api.json`（各服务商的模型）和 `https://models.dev/models.json`
  （各厂商的模型，补充没有服务商列出、但账号可以调用的模型），保存在设备本地的
  `runtime/cache/models_dev/`，不参与 Space 同步。实现见
  `core/crates/runtime/application/src/services/ModelCatalogService.rs`。
- 本地目录超过 24 小时视为过期。用户拉取模型时，过期的目录会先重新下载；进程运行期间每小时
  检查一次。
- 下载失败时继续使用本地已有的目录，失败原因通过 `getModelCatalogStatus` 提供给界面；
  30 分钟内不会自动重试，用户可以手动重试（`refreshModelCatalog`）。
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

## 命令行

```text
operit2 model catalog-status
operit2 model catalog-refresh
operit2 model thinking <provider-id> <model-id> [off|enabled|minimal|low|medium|high|xhigh|max]
```

`provider-model-available-list` 的输出会列出每个模型识别成的目录模型。
