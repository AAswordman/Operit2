import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

const root = new URL("../../../../../", import.meta.url);
/** Reads live Rust production source for explicitly static lifecycle and data-provenance checks. */
function source(relative) {
  return readFileSync(new URL(relative, root), "utf8").replace(/\r\n/g, "\n");
}

/** Extracts one declaration or method region by exact neighboring markers without compiling Rust. */
function region(text, startMarker, endMarker) {
  const start = text.indexOf(startMarker), end = text.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, startMarker);
  return text.slice(start, end);
}

/** Confirms the sole production support constructor captures the actual fully initialized holder before publication. */
test("production startup binds the actual holder delegate before publishing runtime services", () => {
  const application = source("core/crates/runtime/application/src/core/application/OperitApplication.rs");
  const initialization = region(application, "let initializedChatRuntimeHolder =", "        Self {");
  assert.match(initialization, /ChatRuntimeHolder::newWithRuntimeDependencies\(\s*chatFileSystemHost,\s*toolHandler\.clone\(\),\s*providerRuntimeContext\.clone\(\),\s*\)/);
  assert.equal([...initialization.matchAll(/\.bindChatHistoryManager\(/g)].length, 1);
  assert.match(initialization, /\.bindChatHistoryManager\(\s*initializedChatRuntimeHolder\s*\.chatHistoryManager\(\)\s*\.expect\([^;]+?\)\s*\.clone\(\),\s*\)/);
  const bind = initialization.indexOf(".bindChatHistoryManager("), publish = initialization.indexOf("*chatRuntimeHolder");
  assert.ok(bind < publish);
  assert.match(initialization, /\.expect\("new chat runtime holder must be unlocked"\) = initializedChatRuntimeHolder;/);
  assert.doesNotMatch(initialization, /ChatHistoryManager::|RuntimeStorePaths|runtimeRootDir|workspaceRootDir|defaultRuntime|\.await/);
});

/** Checks the injected value is a clone of the exact existing delegate, not a independently opened manager or an on-demand core. */
test("holder exposes a borrowed canonical manager without opening stores or creating replacement cores", () => {
  const holder = source("core/crates/runtime/application/src/core/chat/ChatRuntimeHolder.rs");
  const borrowed = region(holder, "pub(crate) fn chatHistoryManager(", "    /// Returns the core for a slot");
  assert.match(borrowed, /Result<&operit_store::repository::ChatHistoryManager::ChatHistoryManager, String>/);
  assert.match(borrowed, /self\.cores\s*\.get\(&ChatRuntimeSlot::MAIN\)\s*\.map\(\|core\| &core\.chatHistoryDelegate\.chatHistoryManager\)/);
  assert.match(borrowed, /Chat runtime has no initialized main record manager/);
  assert.doesNotMatch(borrowed, /::default\(|getInstance\(|getCore\(|new\(|\.lock\(|try_lock|\.await/);
  const support = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const captured = region(support, "pub fn bindChatHistoryManager(", "    /// Returns services bound to this runtime support instance.");
  assert.match(captured, /self\.chatHistoryManager\s*\.set\(manager\)/);
  assert.match(captured, /self\.chatHistoryManager\s*\.get\(\)/);
  assert.doesNotMatch(captured, /defaultRuntime|::default\(|getInstance\(|RuntimeStorePaths|chatRuntimeHolder|try_lock/);
});

/** Statically follows clone ownership to the shared connection and execution guard without claiming Rust runtime execution. */
test("captured canonical clone shares connection and lease state with Core generation protection", () => {
  const manager = source("core/crates/persistence/store/src/repository/ChatHistoryManager.rs");
  const sync = source("core/crates/persistence/store/src/sync/SqlChatSyncStore.rs");
  const store = source("core/crates/persistence/store/src/SqliteStore.rs");
  const extensions = source("core/crates/persistence/store/src/repository/ChatHistoryExtensions.rs");
  const mutation = source("core/crates/persistence/store/src/sync/SqlChatExtensionTransactions.rs");
  const core = source("core/crates/runtime/application/src/services/core/MessageCoordinationDelegate.rs");
  assert.match(manager, /#\[derive\(Clone\)\]\s*pub struct ChatHistoryManager\s*\{\s*database: Arc<AppDatabase>/);
  assert.match(manager, /syncStore: SqlChatSyncStore/);
  assert.match(sync, /#\[derive\(Clone\)\]\s*pub struct SqlChatSyncStore\s*\{\s*store: SqliteStore/);
  assert.match(sync, /store:\s*database\.store\(\)\.clone\(\)/);
  assert.match(store, /#\[derive\(Clone\)\]\s*\/\/\/[^\n]*\s*pub struct SqliteStore/);
  assert.match(store, /connection: Arc<Mutex<Box<dyn RuntimeSqliteConnection>>>/);
  assert.match(store, /executionLeases: Arc<Mutex<crate::ChatExecutionLease::ChatExecutionLeaseState>>/);
  assert.match(extensions, /self\.database\.store\(\)\.beginChatExecution\(chatId\)/);
  assert.match(mutation, /self\.store\.withPluginExtensionMutation\(target,/);
  assert.match(core, /\.chatHistoryDelegate\s*\.chatHistoryManager\s*\.beginChatExecution\(&chatId\)/);
});

/** Confirms current-conversation requests use native canonical hydration while history requests retain their exact stored revision source. */
test("EnhancedAI request matches extension schema and does not manufacture historical identity", () => {
  const enhanced = source("core/crates/provider/services/src/chat/EnhancedAIService.rs");
  const current = region(enhanced, "pub async fn resolveChatConfigurationForOptions(", "    /// Resumes model processing");
  assert.doesNotMatch(current, /selection:|roleName|roleCardId|characterName|unwrap_or_default|Map::new|json!\(\{\}\)/);
  assert.match(current, /chatExtension: None/);
  assert.match(current, /messageExtension: None/);
  const actualFields = [...current.slice(current.indexOf("ChatConfigurationRequest {")).matchAll(/^\s*(\w+):/gm)].map(match => match[1]);
  const schema = source("core/crates/provider/services/src/runtime_support.rs");
  const declaration = /pub struct ChatConfigurationRequest\s*\{([^}]+)\}/.exec(schema);
  assert.notEqual(declaration, null);
  const requiredFields = [...declaration[1].matchAll(/pub (\w+):/g)].map(match => match[1]);
  // The native model binding is the sole shorthand field in this actual constructor.
  assert.deepEqual([...actualFields, "defaultModelBinding"].sort(), requiredFields.sort());
  const provider = source("core/crates/runtime/application/src/services/ProviderRuntimeSupportService.rs");
  const resolution = region(provider, "pub async fn resolve(", "    /// Reads only the exact authenticated owner's extension");
  const hydration = region(provider, "fn readConversationExtension(", "    /// Resolves display data for a creation draft");
  assert.match(resolution, /request\.requirePurpose\(ChatConfigurationPurpose::Execution\)\?/);
  assert.match(resolution, /self\.readConversationExtension\(&mut request\)\?/);
  assert.ok(resolution.indexOf("readConversationExtension(") < resolution.indexOf("self.invokeResolve(request).await"));
  assert.match(hydration, /request\.chatExtension = self\s*\.recordSupport\s*\.readChatExtension\(&self\.ownerPackage, &target\)\?/);
  assert.doesNotMatch(resolution + hydration, /unwrap_or_default|Map::new|resolveDraft|or_else/);
  const core = source("core/crates/runtime/application/src/services/ChatServiceCore.rs");
  assert.match(core, /PluginExtensionTarget::Message\s*\{\s*chatId: chatId\.clone\(\),\s*messageTimestamp,\s*variantIndex,/);
  assert.match(core, /readPluginExtension\(api\.extensionOwner\(\), &target\)/);
  assert.match(core, /messageExtension: Some\(snapshot\)/);
});