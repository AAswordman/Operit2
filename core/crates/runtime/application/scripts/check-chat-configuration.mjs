import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = new URL("../../../../../", import.meta.url);
let checks = 0;

/** Reads one explicit repository source without searching or rewriting production files. */
function source(path) { return readFileSync(fileURLToPath(new URL(path, root)), "utf8"); }

/** Counts only assertions that actually execute against the current working-tree source. */
function verify(condition, message) { assert.ok(condition, message); checks += 1; }

/** Isolates one owned function using its exact documented adjacent source boundary. */
function section(text, start, end) {
  const first = text.indexOf(start);
  assert.notEqual(first, -1, `Missing function marker: ${start}`);
  const last = text.indexOf(end, first + start.length);
  assert.notEqual(last, -1, `Missing adjacent marker: ${end}`);
  return text.slice(first, last);
}

const application = "core/crates/runtime/application/src/";
const provider = "core/crates/provider/services/src/";
const ownedConsumers = [
  "services/core/ChatHistoryDelegate.rs", "services/core/MessageCoordinationDelegate.rs",
  "services/core/MessageProcessingDelegate.rs", "services/ChatServiceCore.rs",
  "services/ProviderRuntimeSupportService.rs", "services/TtsPlaybackService.rs",
  "services/TtsSynthesisService.rs", "data/preferences/TtsConfigManager.rs",
];
const withdrawn = /\b(?:CharacterCardManager|ActivePromptManager|SharedMemoryStoreManager|MemoryRepository|MemoryManagementService|ChatMemoryOwnerResolver|resolveRoleCardId)\b/;
for (const file of ownedConsumers) verify(!withdrawn.test(source(application + file)), `Old domain consumer: ${file}`);
const history = source(application + ownedConsumers[0]);
const core = source(application + "services/ChatServiceCore.rs");
const support = source(application + "services/ProviderRuntimeSupportService.rs");
const enhanced = source(provider + "chat/EnhancedAIService.rs");
const coordinator = source(application + "services/core/MessageCoordinationDelegate.rs");
const processor = source(application + "services/core/MessageProcessingDelegate.rs");
const runtime = source(provider + "runtime_support.rs");

for (const [name, text] of [["history", history], ["coordinator", coordinator], ["Core", core]]) {
  verify(!/\.character(?:CardName|GroupId)\b/.test(text), `${name} still interprets legacy role fields`);
}
const ready = section(support, "pub async fn ready(", "/// Returns the authenticated catalog owner");
verify((ready.match(/RuntimePackageManager::readySnapshot\(/g) ?? []).length === 1, "Configuration readiness must resolve one canonical snapshot");
verify(/getEnabledToolPkgContainerRuntimes\(\)/.test(ready) && /runtime\.publicApis/.test(ready), "Owner selection must inspect actual enabled public API registrations");
verify(!/com\.operit\.character_cards|character_cards|HostProvider|PluginStore|callDependency/.test(support), "Native configuration calls must not name a plugin or withdrawn framework");
verify(/match candidates\.len\(\)/.test(support) && /Duplicate chat\.configuration\.resolve/.test(support), "Unknown and duplicate owners must be explicit errors");
verify(support.includes('"chat.configuration.resolve"'), "Missing registered resolve API");
verify(!/chat\.configuration\.binding|readBinding|writeBinding|deleteBinding/.test(support + history + core), "Core must not maintain plugin binding mirrors");
verify(/invokeToolPkgPublicApi/.test(support) && !/SoftwareSettings\.exec|execCore|callDependency/.test(support), "Typed native invocations must use the authenticated existing public API bridge");
const create = section(history, "async fn createChatDraft(", "/// Opens a real history and marks");
verify((create.match(/ChatConfigurationApi::ready\(/g) ?? []).length === 1, "Creation must retain exactly one configuration owner");
verify(/ToolPkgChatLifecycleHookBridge::snapshot\(\)\.await\?/.test(create), "Creation must await the ready lifecycle snapshot");
verify(create.indexOf("dispatchBeforeCreate") < create.indexOf("resolveDraft") && create.indexOf("resolveDraft") < create.indexOf("commitChatDraft"), "Creation must complete hook and draft resolve before committing any record");
verify(create.indexOf("commitChatDraft") < create.lastIndexOf("publishChatConfiguration"), "Creation cannot publish before atomic commit");
verify(/sourceChatId: sourceChatId\.clone\(\)/.test(create) && /sourceMessageTimestamp/.test(create), "Lifecycle must receive genuine explicit source and branch point");
verify(!/configuration\.contextKey|inheritGroup|\.selection\b/.test(create), "Core must not interpret plugin inheritance markers");
verify(/current\.pluginExtensions == draft\.pluginExtensions/.test(create) && /current\.workspaceId == draft\.workspaceId/.test(create) && /hasUserMessage/.test(create), "Empty-chat reuse must compare complete generic draft identity and workspace");
verify(/for \(initial of configuration/.test(create) || /for initial in &configuration\.initialMessages/.test(create), "Creation must persist exactly the ordered initialMessages explicitly declared by the plugin");
verify(/if let Some\(snapshot\) = &initial\.messageExtension/.test(create) && /opening\.pluginExtensions\.insert/.test(create), "Only explicit optional initial-message snapshots may be persisted under the authenticated owner");
verify(!/profile\.openingStatement|participants\.first|configuration\.profile/.test(create), "Core must not infer opening statements or select an execution participant during draft display");
verify(/purpose: ChatConfigurationPurpose::Display/.test(create) && /resolveDraft/.test(create), "Drafts must resolve the explicit display purpose");
verify(!/switchConfiguredChat|publishChatConfiguration/.test(processor), "Message execution must not publish an execution descriptor into the presentation cache");
const startup = section(history, "pub fn initialize(", "/// Loads the newest indexed display window");
verify(/openChatHistory\(chatId, false\)\?/.test(startup) && !/switchConfiguredChat|\.resolve\(|ChatConfigurationApi/.test(startup), "Unbound startup must restore persisted history without resolving execution configuration");
const historyOpen = section(history, "pub fn openChatHistory(", "/// Requires an actual Core chat record");
verify(/requireChatExists/.test(historyOpen) && /configurations\.remove\(&chatId\)/.test(historyOpen) && /openChatRecord/.test(historyOpen), "History navigation must open real records and represent configuration as unresolved");
verify(!/ChatConfigurationApi|\.resolve\(|pluginExtensions|defaultModelBinding/.test(historyOpen), "History navigation must not interpret namespaces or synthesize a default profile");
const recordOpen = section(history, "fn openChatRecord(", "/// Initializes a true source branch");
verify(!/chatConfigurationsFlow|\.resolve\(/.test(recordOpen) && /requireChatExists/.test(recordOpen), "Canonical record opening must not require an execution descriptor");
const coreStartup = section(core, "pub async fn initializeChatConfiguration(", "fn syncTokenStatisticsForCurrentChat(");
verify(/self\.chatHistoryDelegate\.initialize\(\)/.test(coreStartup) && !/enhancedAiService|initialize\([^)]*\)\.await/.test(coreStartup), "Core startup must call the synchronous history initializer without an AI runtime");
verify(/if let Some\(result\) = &self\.chatConfigurationInitialization/.test(coreStartup) && /Some\(result\.clone\(\)\)/.test(coreStartup), "Startup must retain its actual success or storage error without retry or error masking");
for (const [start, end] of [["pub async fn switchChat(", "pub async fn switchChatLocal("], ["pub async fn switchChatLocal(", "/// Resolves and publishes only generic display identity"]]) {
  const navigation = section(core, start, end);
  verify(/openChatHistory/.test(navigation) && !/enhancedAiService|switchConfiguredChat|\.resolve\(/.test(navigation), "Both public history switch routes must work without configuration or an AI runtime");
}
const explicitConfiguration = section(core, "pub async fn chatConfiguration(", "/// Resolves the exact persisted message");
verify(/resolveDisplay/.test(explicitConfiguration) && /purpose: ChatConfigurationPurpose::Display/.test(explicitConfiguration) && /\.await\?/.test(explicitConfiguration), "Public presentation resolve must propagate real display API errors without requiring an execution profile");
verify(/ChatConfigurationDisplayResult/.test(explicitConfiguration) && !/resolveChatConfigurationForOptions|\.profile/.test(explicitConfiguration), "Public display results must remain independently typed");
verify(/pub purpose: ChatConfigurationPurpose/.test(runtime) && !/serde\(default\)[^;]*purpose/s.test(runtime), "Every request must carry an explicit required purpose");
verify(/purpose: ChatConfigurationPurpose::Execution/.test(enhanced), "Actual sends must resolve the execution purpose");
const lifecycle = source(application + "plugins/toolpkg/ToolPkgChatLifecycleHookBridge.rs");
const newValidation = section(lifecycle, "ToolPkgChatCreationKind::New =>", "ToolPkgChatCreationKind::Branch =>");
verify(!/sourceChatId\.is_some/.test(newValidation) && /sourceMessageTimestamp\.is_some/.test(newValidation) && /parentChatId\.is_some/.test(newValidation), "New creation must accept a real source but reject branch-only context");
verify(/loadChatHistory\(id\.clone\(\)\)/.test(create) && create.indexOf("Source chat does not exist") < create.indexOf("dispatchBeforeCreate"), "Actual nonempty source records must be read and validated before the hook");
const inputBridge = source(application + "plugins/toolpkg/ToolPkgChatInputHookBridge.rs");
verify(!/已跳过并继续|timeoutNoticeMessage|decodeToolPkgHookResult|unwrap_or_default/.test(inputBridge), "Submit failure must never degrade to a normal send or malformed-result no-contribution");
verify(/Result<Option<ChatInputHookResult>, String>/.test(inputBridge) && /map_err\(\|error\| format!\("Chat input/.test(inputBridge), "Hook execution must preserve errors through its real caller");
const submit = section(core, "pub async fn dispatchChatInputSubmitRequested(", "/// Accepts a host-authored message");
verify(/attachments: Vec<AttachmentInfo>/.test(submit) && /replyToMessageTimestamp: Option<i64>/.test(submit) && /\.await\?/.test(submit), "The public widget submit route must carry actual attachments and reply identity and propagate errors");
verify(/\"attachments\": context\.attachments/.test(inputBridge) && /\"replyToMessageTimestamp\": context\.replyToMessageTimestamp/.test(inputBridge), "Production hook payload must preserve original attachment records and reply target");
const branch = section(history, "pub async fn createBranch(", "/// Loads branches");
verify(/ToolPkgChatCreationKind::Branch/.test(branch) && /createChatDraft/.test(branch), "Branches must use the same real lifecycle and draft transaction");
verify(/pub async fn currentChatIdFlow/.test(core) && /self\.initializeChatConfiguration\(\)\.await\?/.test(core), "Startup flow must await initialization");
verify(/pub async fn chatConfigurationForMessage/.test(core) && /PluginExtensionTarget::Message/.test(core) && /messageExtension: Some\(snapshot\)/.test(core), "Historical profiles must read the exact persisted message revision snapshot");
verify(/applyConfigurationSnapshot\(&mut aiMessage, &configuration\)/.test(processor), "Every generated revision requires its fresh execution snapshot before persistence");
verify(/beginChatExecution\(&chatId\)/.test(coordinator) && /executionLease: Option<ChatExecutionLease>/.test(processor), "Generation protection must be acquired and retained by the real streaming runtime");
verify(/runtime\.executionLease\.take\(\)/.test(processor) && /protectRevision/.test(processor), "Completion/cancel must release guards protecting exact dirty revisions");
verify(/runtime\.chatConfiguration\.profile\.id/.test(enhanced) && /"executionContext"/.test(enhanced), "Prompt hooks must receive the resolved execution participant");
verify((enhanced.match(/let availableTools = applyToolPromptComposeHooksToAvailableTools\(/g) ?? []).length === 3, "Initial sends, subsequent tool rounds and estimates must actually filter model-visible tools");
verify(/options\.executionParticipantId = Some\(configuration\.profile\.id/.test(enhanced), "Tool execution context must use the same resolved participant identity");
verify(/getServiceBundleForModel\([\s\S]*profile\.modelBinding/.test(enhanced), "The actual provider/model service bundle must come from the resolved profile");
verify(/let introPrompt = profile\.introPrompt/.test(enhanced) && /let userPreferencesText = profile\.userPreferencesText/.test(enhanced), "Profile prompt content must be passed to the real runtime");
verify(/chatConfiguration: ChatConfigurationResult/.test(processor) && /sendMessageWithConfiguration/.test(source(application + "core/chat/AIMessageManager.rs")), "The processor and provider must share the resolved descriptor rather than invoke another command");
verify(/stage: "build_tool_prompt"/.test(source(provider + "chat/config/SystemToolPrompts.rs")), "System tool prompts must invoke registered policy with their actual catalog");
verify(!/build_active_prompt_hook_metadata|"activePrompt"/.test(source(provider + "chat/enhance/InputProcessor.rs")), "Input prompt hooks must not classify generic participants as native characters");
verify(!/memorySearchConfig|memoryOwnerKeyForCharacterCard|memoryAutoSaveMessages|ProviderMemoryAutoSaveMessage/.test(runtime), "The provider trait must not expose withdrawn native memory business methods");
verify(!/enqueueAutoSaveCandidate|MemoryLibrary/.test(enhanced) && !/MemoryAutoSaveScheduler|MemoryManagementService::new/.test(source(application + "core/application/OperitApplication.rs")), "Old background memory runtime consumers must actually be removed");
verify(!/pub mod ChatMemoryOwnerResolver/.test(source(application + "services/core/mod.rs")) && !/pub mod MemoryManagementService/.test(source(application + "services/mod.rs")) && !/pub mod library/.test(source(provider + "chat/mod.rs")), "Withdrawn old algorithms must not remain mounted");
verify(/pub fn speakWithConfig/.test(source(application + "services/TtsPlaybackService.rs")) && /pub fn synthesizeWithConfig/.test(source(application + "services/TtsSynthesisService.rs")), "TTS production services must accept explicit resolved configuration identities");
const inputDispatch = section(inputBridge, "pub async fn dispatchChatInputHooks(", "/// Requires the process-owned runtime");
verify(/readySnapshot/.test(inputDispatch) && /getEnabledToolPkgContainerRuntimes/.test(inputDispatch), "Input dispatch must inspect the actual ready enabled registry");
verify(/remainingTimeoutMillis\(\)[\s\S]*ok_or_else/.test(inputDispatch) && /budget\.hasExpired\(\)[\s\S]*return Err/.test(inputDispatch), "Both timeout boundaries must reject the actual submit");
verify(!/Err\([^)]*\) =>[\s\S]*None|CHAT_INPUT_SUBMIT_ACTION_ALLOW\.to_string\(\)/.test(inputDispatch), "An invoked error or expired budget must never produce no-contribution or allow");
const startSend = section(core, "pub(crate) async fn startUserMessage(", "/// Commits complete original input exactly once");
verify(/let submitDecision = submitDecision\?/.test(startSend) && /CHAT_INPUT_SUBMIT_ACTION_BLOCK \| CHAT_INPUT_SUBMIT_ACTION_CONSUME/.test(startSend), "Actual direct-send hook errors must stop before the coordinator while ownership decisions remain explicit");
verify(/MessageSendOutcome::Consumed/.test(startSend) && /ChatTurnSubmission::Handled/.test(startSend), "Consumed input must not be reported as a completed model generation");
verify(!/roleCardId|role_card_id|groupOrchestration|groupParticipant|isGroupOrchestration/.test(core + coordinator + processor + source(application + "core/chat/AIMessageManager.rs")), "Core execution must not retain role aliases or native group interpretation");
const continuation = section(coordinator, "pub async fn sendMessageInternal(", "/// Starts a user-requested conversation summary");
verify(/loadChatMessageVariant\(&chatId, continuation\.userMessageTimestamp, 0\)/.test(continuation) && /source\.sender != \"user\"/.test(continuation), "Continuation must read and validate the precise committed user record");
verify(/request\.turnOptions\.continuation\.is_none\(\)/.test(processor), "Continuation must not enter the actual user insert path");
const commitUser = section(history, "pub(crate) fn commitUserMessage(", "/// Commits one assistant segment");
verify(/recordCommittedUser/.test(processor) && /commitUserMessage\(&chatId, userMessage\.clone\(\)\)/.test(processor)
  && /loadChatMessageVariant\(chatId, persisted\.timestamp, 0\)/.test(commitUser), "User receipts must come from canonical persisted rows");
verify(/recordCommittedAssistant\(&completionChatId, completionTurnId, &completedMessage, 0\)/.test(processor), "Assistant receipts must follow the actual final commit for the originating turn");
const toolsRuntime = source(application + "services/ToolRuntimeSupportService.rs");
const toolsSend = section(toolsRuntime, "fn sendChatMessage<'a>(", "/// Calls the configured functional model");
verify(toolsSend.indexOf("dispatchRegisteredChatInputHooks(context.clone()).await?") < toolsSend.lastIndexOf("self.chatRuntimeHolder.lock().await"), "Tool submit hooks must execute outside the Core Holder lock");
verify(/submission\.wait\(\)\.await/.test(toolsSend) && toolsSend.indexOf("holder.observeStats") < toolsSend.indexOf("submission.wait"), "Tool results must await the originating receipt after releasing Holder");
const standardChat = source("core/crates/tool/services/src/tools/defaultTool/standard/StandardChatManagerTool.rs");
verify(!/latestAssistantMessage/.test(standardChat) && /ToolResultData::MessageSendResultData\(result\)/.test(standardChat), "Tools must return the native receipt instead of guessing recent assistant history");
const nativeTransport = source(application + "services/core/ChatTurnTransport.rs");
const nativeWorker = section(toolsRuntime, "async fn runAdmittedChatTurn(", "/// Reports cancellation before execution");
const sdkChat = source("core/crates/plugin/sdk/src/js_sdk/chat.rs");
const sdkResults = source("core/crates/plugin/sdk/src/js_sdk/results.rs");
const nativeRecordOnly = section(core, "pub(crate) fn recordOnlySequenceInput(", "/// Cancels only the native execution");
const nativeHost = source("core/crates/tool/services/src/tools/AIToolHandlerJsToolsHost.rs");
verify(/fn sendMessage\(&self, request: ChatSendRequest\)/.test(sdkChat)
  && /fn sendMessageStreaming\(&self, request: ChatSendRequest\) -> JsAsyncIterable<ChatSendEvent>/.test(sdkChat)
  && /fn cancel\(&self, chatId: String\)/.test(sdkChat), "Authors receive only single send, semantic streaming send and owner/chat cancellation");
verify(!/fn (startTurn|waitTurn|cancelTurn|finishSequence)\(|onIntermediateResult|ChatSendMessageStreamingOptions/.test(sdkChat), "Native execution handles and callback streaming must not leak into the current public contract");
verify(/fn openPluginChatMessage\(/.test(toolsRuntime) && /fn requestPluginChatCancellation\(/.test(toolsRuntime), "The new public methods require actual owner-authenticated runtime implementations");
verify(!/ChatTurnHandle|ChatSequenceFinishResultData/.test(sdkResults) && /uuid::Uuid::new_v4/.test(nativeTransport), "Only native admission generates execution handles; the author result contract does not expose them");
verify(/manager\.beginChatExecution\(&chatId\)/.test(nativeTransport) && /lease: lease\.clone/.test(nativeTransport), "Native admission retains the canonical store lease through finalization");
verify(/execution\.owner != owner/.test(nativeTransport) && /sequence\.owner != owner/.test(nativeTransport), "Execution and sequence operations must reject foreign authenticated owners");
for (const helper of ["open_chat_send", "invoke_chat_cancel"]) {
  const helperBody = section(nativeHost, `fn ${helper}(`, "\n}");
  verify(/authenticated_chat_extension_owner\(host\)/.test(helperBody) && /host\.runtimeSupport\(\)/.test(helperBody), `${helper} must use actual immutable host authentication and runtime support`);
}
verify(/ownedActiveExecution\(owner, chatId\)/.test(toolsRuntime)
  && /sequence\.owner == owner && sequence\.chatId == chatId/.test(nativeTransport), "Cancellation captures the actual authenticated owner/chat execution before asynchronous model access");
verify(/source\.sender != "user"/.test(nativeTransport) && /Continuation source was superseded/.test(nativeTransport), "A continuation must identify a real nonsuperseded committed user row");
verify(/ChatSendRequest::Continue/.test(nativeTransport) && /AdmittedChatTurnWork::Continue \{ userMessageTimestamp, participantId \}/.test(nativeTransport), "Continuation admission never resubmits replacement input or attachments");
verify(/requestCancellation/.test(nativeTransport) && /cancelled\.store\(true, Ordering::Release\)/.test(nativeTransport), "Cancellation covers the admitted-before-start interval");
verify(/nativeExecutionId\.as_deref\(\) == Some\(executionId\)/.test(processor) && /isNativeExecutionActive\(chatId, executionId\)/.test(core), "Native cancellation must match the precise runtime execution before touching the model");
verify(/commitUserMessage/.test(nativeRecordOnly) && /protectRevision/.test(nativeRecordOnly) && /buildRecordedUserMessageContent/.test(nativeRecordOnly), "Record-only must use checked canonical persistence and the real sequence lease");
verify(!/resolveChat|initializeChatConfiguration|newEnhanced|modelConfig|\.sendMessage/.test(nativeRecordOnly), "Record-only must not resolve a model, synthesize a profile, or start generation");
verify(/attachments\.clone\(\), input\.replyToMessageTimestamp/.test(nativeWorker) && /dispatchRegisteredChatInputHooks\(context\.clone\(\)\)\.await\?/.test(nativeWorker), "Native submit must preserve complete attachments and reply identity while propagating hook errors");
verify(/ChatInitialTurn::RecordOnly/.test(nativeWorker) && /core\.startUserMessage/.test(nativeWorker) && /submission\.wait\(\)\.await/.test(nativeWorker), "Native record-only and execution must have distinct real persistence paths with one originating terminal wait");
verify(/nativeCommittedTurnReceipt/.test(toolsRuntime) && /committedReceipt/.test(nativeTransport), "Native failed sends must retain exact committed locators rather than losing persistence receipts");
verify(/finalizeSequenceAndNotify/.test(core) && /prepareFinish/.test(toolsRuntime) && /completeFinish/.test(toolsRuntime), "Host finalization publishes actual send completion and releases its canonical lease without an author finish call");
verify(/!turnOptions\.deferSequenceCompletion/.test(processor) && /!runtime\.currentTurnOptions\.deferSequenceCompletion/.test(processor), "Per-turn completion must not release a sequence lease or duplicate sequence-level notifications");
const semanticObserver = source(application + "services/core/ChatSendStream.rs");
verify(/MAX_CHAT_SNAPSHOT_BYTES: usize = 8 \* 1024 \* 1024/.test(semanticObserver)
  && /watch::channel\(ObservationState::default\(\)\)/.test(semanticObserver), "Semantic observation uses a bounded latest-state mailbox, not an unbounded event queue");
verify(/parser\.pushSnapshot\(&snapshot\)/.test(processor) && /parser\.resetToSnapshot\(&snapshot\)/.test(processor), "Normal chunks and response revisions update the same authoritative native semantic parser");
verify(/observation\.finish\(finalized\)/.test(toolsRuntime) && /finalizeAdmittedChatSend/.test(toolsRuntime), "The real terminal event follows host-owned finalization and lease release");
verify(!/character_cards|HostProvider|PluginStore|roleCardId|groupOrchestration/.test(nativeTransport + nativeWorker), "Native transport must not contain plugin IDs, withdrawn frameworks or old domain aliases");
console.log(`Chat configuration source invariants passed: ${checks} executed assertions.`);
console.log("SCOPE: source checks only; no Rust type-check, production plugin invocation, group planning or native memory job acceptance; persisted message identity has separate SQLite/repository tests.");
