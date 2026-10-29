# Character cards and their memories

## Status: extraction in progress, not a production replacement

The package owns character cards and memory in one ToolPkg
(`com.operit.character_cards`). Memory stays inside the character workflow.
The offline Material UI is present, but passing Node or browser tests does **not**
mean the application is connected or the original feature has been fully extracted.

## Compose route entries

Each of the nine UI routes has its own default-exported entry at
`src/ui/<route-id>/index.ui.ts`, built as `dist/ui/<route-id>/index.ui.js`.
The main bundle keeps these imports external and does not re-export the screen
functions: the SDK serializes a screen's module path, not its named export, and
root export tagging would otherwise overwrite the independently loaded module
identity. The build includes all nine executable screen modules in the `.toolpkg`
archive alongside the main provider and memory tools.

The route regression tests execute the actual JavaScript registration and Compose
SDK scripts, check the serialized paths and render each entry with explicit input.
Installed module bytes and the Core asset archive are compared against the current
build. The embedded chat sidebar uses native Compose DSL rows, search, group menus
and dialogs; it does not instantiate a WebView. The obsolete browser sidebar source, resource
registration and packaging entry have been removed. Role categories contain their
conversation groups, so this package contributes only the legacy Characters tab;
the native Workspace tab remains owned by Flutter. These are Node/script checks, not a live native-host acceptance result.

## Plugin-owned sidebar indicators

The sidebar's vertical rail is composed in `src/ui-sidebar-native-layout.ts` from
ordinary `Box` nodes, not a host `HistoryRail` widget. The plugin owns its baseline,
indicator sizes, theme colors, corner radius and selected-state projection.
`Box.hoverStyle` supplies partial visual overrides from the enclosing `HoverRegion`
(including an open row menu); `Box.animation` supplies the duration and easing curve.
The host interpolates those generic visual properties without JS callbacks per frame.
Stable node keys and an unchanged wrapper hierarchy preserve in-flight transitions
and ink ripples when chat selection refreshes. Flutter's own Workspace drawer is
independent and may keep using its internal rail widget.

## Host sidebar state and detached windows

The host supplies `chatSidebar.currentChatId` only while its chat route is active;
other destinations supply `null`, clearing the role sidebar's selected row without
changing the actual current conversation or reloading the role catalog.

`chatSidebarCapabilities.openInWindow` is separate host-supplied Compose input.
Only an explicit `true` exposes “在新窗口打开” in the long-press dialog. The action
returns exactly `{ type: "toolpkg.chat.openInWindow", chatId }`; Flutter validates
the live registration and conversation, then uses its existing desktop launcher
and current theme snapshot without activating the chat in the original window.
Missing capability input (including older hosts) keeps this desktop-only action
hidden. The canonical sidebar metadata/catalog contract is unchanged.

## Chat input menu

The package registers one `chat_input_menu` navigation entry pointing to
`src/ui/chat-input-menu/index.ui.ts`. Flutter embeds that Compose DSL module
above its native statistics section. The TS renderer owns the avatar, current
character label, name, spacing and click action; the public chat-context API
contains only identity and background data.

The memory group registers on `chat_input_menu_sections` and renders below native
statistics, before Tools, through `src/ui/chat-input-memory/index.ui.ts`. The plugin
owns its original summary, two switches, manual extraction and three-second queue
polling. `memory-status` provides the original status dialog through the same
generic presentation protocol. Preferences, owner resolution and extraction all
use the existing plugin service and Storage; Flutter carries no memory rules.

The row emits `{type: "toolpkg.ui.present", routeId, input}` to open the existing
registered selector. The host validates the requesting package and route, keeps
opaque input unchanged, and uses its existing chat lifecycle guard and
presentation completion refresh. Character menu fields are drawn by TS.

## Character settings scope

The plugin-menu character settings follow the native settings page immediately
before commit `824dcb7c`: character/group lists; basic, content and binding editor
tabs; model/TTS/memory/tool bindings; and existing import/export operations.
The character page's entire inner title bar (icon, title and refresh action) is
removed; the character/group section titles remain. Character and group editors
retain the independent theme reference picker. Extra group preview/duplicate edit
buttons, member reordering actions and the implementation notice remain removed.
Group rows show member names as their subtitle and only a member-count badge.
Theme references remain editable and are preserved by persistence/interchange.
The picker reads actual named configurations through `Tools.Theme.listConfigs()`
via the main-runtime `listThemeChoices` IPC action. It opens immediately with a
cancellable loading state and explicit retry on failure; an empty directory is
not substituted for a failed request. Selecting stages only the editor's
`themeConfigId`; saving persists it, and activating the character/group applies
the exact configuration through `Tools.Theme.applyConfig(id)`.
Null means keep the current theme, and groups never borrow a member's theme.
Character/group creation, editing and previews are authored once in
`src/ui-entity-native.ts`, including their binding, tag and export subpanels.
Read-only previews use `src/ui-entity-preview.ts` for a continuous reading layout:
portrait and identity, attached tags, opening statement and populated text sections,
with named configuration summaries behind one expandable row. Group previews show
ordered member portraits and descriptions. Edit mode retains the main-style tabs.
Management's WebView list delegates via `openEntity`; its old entity dialog
renderers and contracts have been removed. Closing the native surface emits
`character-entity-closed` so management reloads genuine records, retaining its
WebView instance. Memory/import dialogs remain independent. Browser tests can run with
`CHARACTER_CARDS_UI_BROWSER=webkit node --test tests/ui.test.mjs`.
Chat identity previews remain available from avatars through the explicitly
scoped `identity.avatarAction`. Application/chat title text is display-only and
never opens that preview. The independent memory page is unchanged.

## Current Chat integration review

The current send/control contract is documented in `docs/chat-send-api.md` at the
repository root: one non-streaming send, one semantic async iterable, and one
owner/chat cancellation method. The group transport no longer exposes native
execution handles or requires author-owned native finalization. Generic SDK
lifecycle/settings/JSON declarations now match the current role-plugin source.

Strict host/browser no-emit checks, explicit architecture checks and routing checks
pass. The direct group transport tests use controlled native receipts and frozen
message snapshots; they do not establish a live Core execution lease or configured
AI response. The existing group fixture was brought onto the actual two-argument
controller factory and its explicit local planning lifecycle.

The main registration does **not** yet mount `GroupExecutionController` in a chat
input-submit hook. Installed production archives also remain different from
current source until packaging is explicitly performed. Neither gap is described
as completed integration.

## Plugin-owned group prompts and configuration deletion

Group execution adds identity constraints and the saved ordered member names to each
participant's `introPrompt` inside the registered configuration resolver. The same
profile is frozen in the message snapshot, so later provider rounds and historical
execution retain its exact prompt. Single-card execution remains scoped to the
card's own authored prompt. Core no longer carries group-orchestration options or
role-specific prompt builders. The existing functional model slot used by the
planner remains an independent host configuration capability.

The authenticated `chat.configuration.delete` API releases plugin-owned bindings
after Core deletes a conversation and returns an exact completion acknowledgement.
Historical snapshots remain available. Core does not call binding business APIs.
CLI help and dispatch read enabled commands from the existing Core plugin catalog;
role command names and descriptions are supplied only by this package.

## Storage and released-data migration

The request path is:

- UI, commands and public APIs enter `service-runtime.ts`, which opens one lazy service.
- Reads enter `storage/database.ts`. `operations.ts` routes configuration edits to preference snapshots and memory operations to `storage/memory-repository.ts`; HTTP calls hold no repository-wide queue.
- Configuration records live in native JSON preferences; memories and jobs live in `memory.sqlite`. USER.md stays a file.
- `drafts.ts:readCharacter` decodes persisted and imported cards with the same defaults as new cards. It returns a new value and preserves existing content.

There is no startup repair scan, whole-database record validation, or second service
adapter. Missing fields are completed when read and persisted with the next normal
write. A damaged unrelated memory job cannot block reading or editing a character.
Released-data migration uses this same decoder before publishing its records.

The unreleased plugin's former `state.json` format is removed without a compatibility
reader. Released application data takes a distinct migration path: Core's centralized
`data/backup/CharacterPluginMigration.rs` reads old preferences and SQLite records,
uses the production generic Host object store, verifies the copied documents and
resources, resolves saved chat associations, and deletes migrated legacy owners and
the four dedicated preference files. An interrupted migration blocks plugin reads;
completed imports and cleanup have durable markers.

Operit1 snapshot restore uses the same writer directly. It adopts characters,
groups, prompt tags, memory profiles, USER.md, memories, properties, document chunks,
links, pending extraction tasks and declared memory preferences. Source Room schemas
remain explicit; chat name/group bindings resolve to plugin selection markers before
Core chat import. Historical message send profiles are not reconstructed. Derived
binary embedding vectors are not imported. Explicit rebuilds derive current source vectors using the configured provider.
See `docs/migration-acceptance.md` and `docs/operit1-restore-boundary.md` for evidence.

## Ownership boundary

- The plugin owns domain models, validation, complete-record writes, staged tags,
  serialization/interchange, commands, public APIs, and editing requests.
- `registerCoreCommand` registers command **providers**, not callers of the old
  CLI executor. Public APIs and Web IPC invoke the same plugin service.
- Core owns reusable command discovery/readiness, routing, IPC, and host IO. It
  must not acquire dedicated character/memory Host services, SDK DTOs, globals,
  runtime adapters, dedicated domain import services, or unapproved domain frameworks.
- Old mounted manager consumers, chat ownership policy, memory background tasks,
  AI send-path memory logic, and builtin memory executors remain extraction
  failures even when the five command roots have been registered.
- Unknown/disabled providers and real dependency failures must return explicit
  errors. No success-shaped empty results, production test fixtures, browser-local
  database, CLI business bridge, or alternate transport are accepted.

## Browser source and packaging

Compose screens under `dist/ui/` ship as readable CommonJS JavaScript with UTF-8
text and their authored UI declarations and callbacks. Users can edit these entry
files and repackage the plugin. The runtime owns Compose compilation and caching;
plugin packaging does not emit versioned Compose artifacts.

Hand-written browser source is strict TypeScript under `web/`. esbuild produces a
browser IIFE inside the offline HTML; generated JavaScript is a packaging artifact,
not a second source implementation. `tsconfig.web.json` checks browser DOM code
separately from the non-DOM host project. UI tests build current TS source with the
build worker's document builder instead of reading stale generated resources.
The archive must include every current nested `src/` and `web/` source module.
The Web/build worker owns `scripts/build.mjs` and the archive.

## Verification and evidence limits

Strict host/browser TypeScript checks run through `scripts/check.mjs`. Repository
and service tests use current plugin code and a generic Node SQLite/file Host adapter;
that adapter implements storage semantics rather than character business operations.
It is distinct from production Rust Host integration.

Rust migration tests use the production `PluginStorageSession`, SQLite and native
storage Hosts. Database migration tests exercise actual schema upgrades, every
transaction failure boundary and complete persisted rows. Operit1 tests cover real
Room source databases and structured settings; they do not constitute a complete
application launch or a full import of a user-supplied ObjectBox archive.

Production artifacts are generated by `scripts/build.mjs`; packaging checks compare
current source, installed bundle and Core archive bytes. Flutter UI and application
startup require their own integration acceptance.

## Public dependency client

`manifest.json` publishes the self-contained client `src/api.ts`. Consumers
copy that file into their own source tree, import it locally and declare
`com.operit.character_cards >= 0.2.0` in `requires`, following
`plugins/docs/PUBLIC_API.md`. Public calls use `ToolPkg.callDependency`.

`characterCards.memory.query({ participantId, query, limit, snapshotId })`
resolves the explicit participant's actual memory binding and reuses the
plugin's existing query implementation. Results contain complete typed
matches and document chunks; snapshots remain in the provider's main runtime.
A `null` snapshot ID requests a fresh query. An explicit snapshot ID retains
owner-scoped deduplication across calls.

## Built-in memory recall

The plugin owns per-turn memory recall through its prompt finalize hook
(`src/memory-recall.ts`). On every main character or group turn it strips host
markup from the authored input, queries the speaking participant's primary
memory owner with a per-chat snapshot for owner-scoped deduplication, and
appends the configured number of matches as a transient `<memory_context>`
block to the final model request. Injections never persist into chat history,
an empty result injects nothing, and provider failures fail closed without
blocking the send.

The memory menu's expanded section controls the persisted `memoryRecall` state
field: the 自动检索注入记忆 switch, a numeric input for the per-turn injection
count (1–50, committing every valid typed value), and the 允许重复命中 switch
that bypasses the per-chat snapshot exclusion. States written by the released
plugin or the Core migration default to enabled, count three, and repeated
hits off. This recall replaces the memory item formerly offered by the extra
information injection package.

Retrieval always honors the owner's persisted search weights from the memory
settings retrieval tab. Disabling cloud embeddings explicitly selects lexical,
tag and graph retrieval. Enabling embeddings requires a valid endpoint and model;
provider errors and an incomplete or stale source-vector index fail the request.
Changing providers or enabling embeddings on an existing library requires an
explicit embedding rebuild.

## Owner memory controls

The memory-settings modal uses the existing `character-memory.request` main-runtime
IPC channel for real owner settings, search weights, bound-chat history, candidate
queue counts and durable rebuild progress. `writeMemoryOwnerSettings` validates and
publishes settings plus search weights in one repository transaction; it preserves
the scheduler's latest `nextAutoSaveRunAtMs` rather than overwriting it from a stale
editor draft. Both character and shared-library rows open their exact owner.

Memory candidates are produced on the device that actually persists a reply; receiving
synchronized history does not create another extraction request. Each candidate records
that producing CoreNode, and each device's ordinary interval processes only its own
requests. History rebuilds likewise remain with the device on which they were requested.
Restarting that device resumes its persisted work; another device never takes work merely
because its producer is unreachable. Memory records and task provenance remain synchronized.
Released tasks without producer metadata are reported as failed without calling a model;
explicit retry assigns candidate work to the requesting device, and a new rebuild records
its requesting device. One owner's failure does not stop independent owners or later intervals.
The modal polls genuine queue/progress values without replacing editable fields, and stops
polling on close without cancelling jobs.
Embedding rebuilds use the saved endpoint/model/key and the existing HTTP capability;
provider errors stay visible. Rebuilds publish version-checked source pages; already
committed pages remain committed when a later page fails. The unnecessary
score-mode selector is omitted while its stored configuration is preserved.

`tests/memory-controls-ui.test.mjs` exercises browser controls through registered
main IPC and production repository/job code with temporary-disk storage, controlled
Chat/AI host inputs, and an actual local HTTP embedding server. It is not a Flutter
or native Rust-engine acceptance test and does not require external credentials.

Remaining memory/editor entry points use the same real main IPC rather than disabled
placeholder controls:

- Global `memoryPreferences` live in the existing v3 metadata transaction. New live
  execution profiles omit USER.md when disabled. Historical re-execution also
  suppresses the runtime profile text, without modifying its saved opaque marker. Disabling automatic updates stops automatic candidate enqueueing
  and scheduler batch claims, but does not cancel explicit history rebuilds.
- Core's migration boundary transfers the original global flags (including
  already-completed v27 migrations) and Operit1 flags once. Existing plugin metadata
  without flags upgrades to its previously active behavior; malformed stored flags
  fail reads instead of guessing values. Provider prompt assembly trusts the plugin
  execution profile, not a second retired Core preference gate.
- Retrieval debugging reuses the production database search and real provider embeddings.
  Draft weights are not saved. Each candidate reports actual lexical/tag/vector/edge
  contributions and its threshold decision; normal retrieval filters the same results.
- Graph export/import invokes the full owner backup codec, including records, document
  chunks, relationships, settings and USER.md. Relationship tools read one owner-scoped
  graph snapshot containing the actual record and link identities and descriptions.
- Character imports select Operit or Tavern JSON; exports invoke the corresponding
  production serializer. Preview avatars use the existing host-loaded image resources.
  Export dialogs only report success after the real host file operation acknowledges it.

Memory nodes and document chunks share lexical and semantic ranking; tools no longer
export full backups or maintain their own embedding/scoring implementation. UI memory
mutations use the same domain operations as dependency clients. Extraction follows the
Kotlin query budgets, duplicate-title prompt and new-identity merge behavior; the
scheduler orders candidates by source timestamp before applying its twenty-record cap.

Kotlin remains the behavior reference, with explicit implementation differences:
lexical expansion uses bounded Unicode terms and Han unigrams/bigrams rather than
Jieba; imported document chunks also participate in node recall; vectors use the
configured HTTP provider and exact database-side dot products over normalized source
vectors. Source text and its vector occupy one synchronized, versioned row; query
vectors are ephemeral. SQL applies owner/folder/time scope, scoring, graph expansion,
snapshot exclusions and result limits before returning records. There is no retained
TypeScript corpus or query-time construction of historical vectors. HTTP batches
contain at most 64 inputs and use at most four concurrent workers. Failed jobs stay
terminal without automatic retry. Merge removes source
chunks and internal links that would become self-links under this domain's invariant.

The browser/main-IPC suite covers these flows, failed writes, runtime reopening,
owner isolation and a real local HTTP vector provider. `cargo check -p operit-runtime
--lib` checks the changed production Rust chain; native-device acceptance still
requires running the updated application.

### 原生实体弹窗

从头像或角色侧栏打开的角色卡/群组预览和编辑由 `src/ui-entity-native.ts` 使用普通 Compose DSL 控件生成，不加载 WebView/HTML，也不复用宿主的角色业务控件。预览、编辑、删除确认和标签草稿在同一个原生 Dialog 中切换；保存使用原有主运行时请求，完成/取消仍使用真实的 PresentationV1 接收方。全屏/还原使用通用 `Dialog.properties.fullscreen`，全屏时移除平台弹窗边距并铺满可用窗口，而非扩大 WebView 内部元素。管理页列表仍保留 WebView，但创建、编辑、预览统一通过 `openEntity` 调用这一套原生实现；旧 WebView 角色卡/群组弹窗及其绑定、标签编辑实现已删除。记忆功能仍保留原有独立入口。

### 可选角色插件与普通模型聊天

角色卡插件关闭时，Core 使用独立的普通模型配置，保留聊天创建、回复、工具执行、语音配置和历史快照能力。没有配置插件注册是有效的应用状态；已注册插件的配置错误和重复注册仍直接报错。原生模型快照在重新启用插件后仍由原生运行层读取，工具策略会验证原生快照，并向其他插件传递隔离后的上下文。

插件启用时，原生选择器提供“仅使用模型”，CLI 提供 `active-prompt set-model`。没有角色关联的现有聊天属于普通模型模式。该模式不注入角色提示词，不授予角色记忆资源，自动记忆队列不为其建立任务。全局无角色状态使用现有可空 active 存储表达，消息中的无记忆目的地使用空 primaryOwnerKey 表达；SQL 表声明不变。

回复者名称使用实际模型名，头像通过明确的 `provider-logo:<providerTypeId>` 标识加载供应商 logo，不按供应商名称猜测。Core 为新消息和开场消息保存通用的显示身份；Flutter 各消息样式读取所选消息版本的身份，不使用当前聊天头像替换历史回复。v1 SDK 保持原设计。


### 群组历史、输入持久化和回合结束摘要

群组在调用 `ROLE_RESPONSE_PLANNER` 前，先通过 `record_only` 保存完整原始输入、附件和回复目标。
输入钩子等待真实入库回执后才清空编辑框；入库失败时保留编辑框原文，规划失败时保留已入库的用户消息。
所有成员回复均继续同一条用户消息，不重复提交输入。空计划仅留下用户消息。

执行配置显式声明 `historyPolicy`。角色卡按已保存的作者 ID 区分历史；群组同时给用户消息添加
`[From user]`。当前角色自己的历史保留 assistant/tool 边界，其他角色的 Markdown 发言以
`[From role: name]` 作为参考上下文，排除其思考、状态和工具协议。姓名来自原消息的显示身份；
改名、重名和成员退出不改变历史作者。缺失作者身份会直接报错。

中间成员使用 `summaryPolicy: "defer"`，最后一个成员使用 `"after_reply"`。每次成员回复期间均不自动摘要；
最后一个成员真实完成且原生发送清理结束后，Core 重新计算各参与者上下文窗口并检查摘要阈值，
达到阈值时通过已有摘要任务执行一次摘要，不继续生成新回复。取消、发送失败和空计划不触发结束摘要。
宿主只处理通用历史和摘要策略，群组计划仍由插件负责。

The current vector engine uses SQLite JSON/window queries through the shared host.
It performs exact scans within the selected scope; it has no ANN/HNSW index and no
sqlite-vec extension. Query latency still includes the configured provider request.
Explicit imports use independently atomic batches and may partially commit; USER.md,
configuration preferences and the memory database have separate transaction boundaries.
Backups export source records without the retired text cache; importing with embeddings
enabled recomputes vectors. Synchronization carries existing vectors with their source
row. Opening migrated libraries deletes retired cache rows without loading their payloads.

### 记忆检索索引

SQLite 保存同步的源记录及其 embedding，检索不再在 SQL 中展开向量 JSON、计算点积或执行多层排名 CTE。
插件第一次读取某个 owner 时，以主键范围分页建立词项索引、标题索引、文件夹和时间索引、关系邻接表，以及 USearch WASM 近邻索引。
运行中的后续查询读取本地变更游标，只更新发生变化的源；普通查询只对候选进行合分，最后按 key 读取结果页。
文档块复用同一索引和分词规则。检索仅生成当前 query 的临时向量，源 embedding 的缺失、过期或维数变化仍要求显式重建。

`vendor/usearch` 是本插件拥有的依赖及构建入口，与 `plugins/sdk` 的外部 IPC 客户端无关。
ANN 是近似检索，文件夹、时间和排除条件传入 WASM 过滤器；候选预算有上限，避免文档大量分块时退化为全库精确扫描。
词项和关系评分在 `src/memory-search.ts` 中实现，保留 RRF、权重模式、阈值和排除后的分页语义。
索引当前保留在运行时内存中，冷启动会重建索引；WASM 保存／恢复能力已由独立示例验证，尚未接入记忆插件的磁盘检查点。
