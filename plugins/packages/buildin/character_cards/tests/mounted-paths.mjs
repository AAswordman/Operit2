import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repository = fileURLToPath(new URL("../../../../../", import.meta.url));
const application = "core/crates/runtime/application/src/core/application/OperitApplication.rs";
const applicationModules = "core/crates/runtime/application/src/core/application/mod.rs";
const services = "core/crates/runtime/application/src/services";
const serviceModules = `${services}/mod.rs`;
const coreModules = `${services}/core/mod.rs`;
const provider = "core/crates/provider/services/src";
const tool = "core/crates/tool/services/src";

/** Reads exact inspected Rust source and removes literals/comments while preserving source line numbers. */
function code(relative) {
  const text = readFileSync(path.join(repository, relative), "utf8");
  return text.replace(/"(?:\\.|[^"\\])*"|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    /** Prevents archived comments and diagnostic strings from becoming invocation evidence. */
    token => token.replace(/[^\r\n]/g, " "),
  );
}

/** Escapes an explicit Rust symbol before constructing an exact lexical check. */
function escaped(symbol) { return symbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

/** Tests an actual module declaration rather than treating every retained reference file as mounted. */
function mounted(moduleFile, symbol) {
  return new RegExp(`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?mod\\s+${escaped(symbol)}\\s*;`, "m").test(code(moduleFile));
}

/** Locates an explicit invocation and excludes ordinary Rust function declarations. */
function firstCall(text, symbol) {
  const expression = new RegExp(`\\b${escaped(symbol)}\\s*\\(`, "g");
  for (const match of text.matchAll(expression)) {
    if (/\bfn\s+$/.test(text.slice(0, match.index))) continue;
    return text.slice(0, match.index).split("\n").length;
  }
  return null;
}

/** Records only real invocation hits in an exact mounted source file. */
function reportCalls(violations, file, symbols, reason) {
  const text = code(file), hits = [];
  for (const symbol of symbols) {
    const line = firstCall(text, symbol);
    if (line !== null) hits.push(`${symbol} at line ${line}`);
  }
  if (hits.length !== 0) violations.push(`${file}: ${reason}: ${hits.join(", ")}`);
}

/** Rejects compiled runtime adapters that still directly invoke old character/memory managers. */
export function runtimeManagerViolations() {
  const violations = [];
  for (const service of ["ProviderRuntimeSupportService", "ToolRuntimeSupportService"]) {
    if (!mounted(serviceModules, service)) continue;
    reportCalls(violations, `${services}/${service}.rs`, [
      "CharacterCardManager::getInstance", "SharedMemoryStoreManager::getInstance", "MemorySearchSettingsPreferences::new",
    ], `mounted by ${serviceModules}, still owns domain manager consumption`);
  }
  if (mounted(serviceModules, "ChatServiceCore")) {
    reportCalls(violations, `${services}/ChatServiceCore.rs`, ["ActivePromptManager::getInstance"], "mounted chat still reads the old active character manager");
  }
  if (mounted(serviceModules, "core") && mounted(coreModules, "MessageCoordinationDelegate")) {
    reportCalls(violations, `${services}/core/MessageCoordinationDelegate.rs`, ["CharacterCardManager::getInstance", "ActivePromptManager::getInstance"], "mounted message coordination still owns old character managers");
  }
  return violations;
}

/** Rejects exact domain members in the two still-exported runtime-support traits and their DTOs. */
export function domainTraitViolations() {
  const violations = [];
  const contracts = [
    { root: provider, methods: ["memorySearchConfig", "memoryOwnerKeyForCharacterCard", "memoryAutoSaveOwnerKeys", "memoryAutoSaveMessagesBefore", "memoryAutoSaveMessagesByTimestamps", "characterPromptContext"], structs: ["ProviderCharacterPromptContext", "ProviderMemoryAutoSaveMessage"] },
    { root: tool, methods: ["resolveCharacterCardToolAccess", "listCharacterCards", "characterCardName", "characterMemoryBinding", "assertMemoryOwnerExists", "loadMemorySearchSettings"], structs: ["RuntimeCharacterCardInfo", "RuntimeCharacterMemoryBinding", "ResolvedCharacterCardToolAccess"] },
  ];
  for (const contract of contracts) {
    if (!mounted(`${contract.root}/lib.rs`, "runtime_support")) continue;
    const file = `${contract.root}/runtime_support.rs`, text = code(file), hits = [];
    for (const method of contract.methods) {
      if (new RegExp(`^\\s*fn\\s+${escaped(method)}\\s*\\(`, "m").test(text)) hits.push(`fn ${method}`);
    }
    for (const symbol of contract.structs) {
      if (new RegExp(`^\\s*pub\\s+struct\\s+${escaped(symbol)}\\b`, "m").test(text)) hits.push(`struct ${symbol}`);
    }
    if (hits.length !== 0) violations.push(`${file}: exported domain trait/DTO contract remains: ${hits.join(", ")}`);
  }
  return violations;
}

/** Rejects mounted old background controls and the application startup scheduler call. */
export function backgroundMemoryViolations() {
  const violations = [];
  if (mounted(applicationModules, "OperitApplication")) {
    reportCalls(violations, application, ["MemoryAutoSaveScheduler::schedule", "MemoryManagementService::new"], "application still invokes old memory lifecycle");
  }
  if (mounted(serviceModules, "MemoryManagementService")) {
    reportCalls(violations, `${services}/MemoryManagementService.rs`, [
      "CharacterCardManager::getInstance", "MemorySettingsRepository::new", "MemoryAutoSaveScheduler::status",
      "MemoryLibrary::saveMemoryNowForOwner", "MemoryLibrary::autoCategorizeForOwner", "MemoryLibrary::saveMemoryWindowNowForOwner",
    ], `mounted by ${serviceModules}, background memory business remains in Core`);
  }
  if (mounted(serviceModules, "ChatServiceCore")) {
    reportCalls(violations, `${services}/ChatServiceCore.rs`, ["MemoryManagementService::new"], "chat consumer still constructs old memory management service");
  }
  return violations;
}

/** Rejects only mounted provider-chat domain calls, not the generic AI service itself or archived references. */
export function providerChatMemoryViolations() {
  const violations = [];
  if (!mounted(`${provider}/lib.rs`, "chat")) return violations;
  const chat = `${provider}/chat`, chatModules = `${chat}/mod.rs`;
  if (mounted(chatModules, "EnhancedAIService")) {
    reportCalls(violations, `${chat}/EnhancedAIService.rs`, [
      "characterPromptContext", "UserMarkdownRepository::new", "MemoryLibrary::enqueueAutoSaveCandidate",
    ], "mounted AI send path still owns character prompts, USER.md, or memory autosave business");
  }
  if (mounted(chatModules, "library")) {
    const modules = `${chat}/library/mod.rs`;
    if (mounted(modules, "MemoryLibrary")) {
      reportCalls(violations, `${chat}/library/MemoryLibrary.rs`, [
        "MemoryRepository::new", "UserMarkdownRepository::new", "memoryOwnerKeyForCharacterCard", "memorySearchConfig",
      ], "mounted background library still reads and writes the old domain repositories");
    }
    if (mounted(modules, "MemoryAutoSaveScheduler")) {
      reportCalls(violations, `${chat}/library/MemoryAutoSaveScheduler.rs`, [
        "memoryAutoSaveOwnerKeys", "memoryAutoSaveMessagesByTimestamps", "memoryAutoSaveMessagesBefore", "MemoryLibrary::saveMemoryNowForOwner",
      ], "mounted scheduler still resolves old owner policy and invokes old memory persistence");
    }
  }
  if (mounted(serviceModules, "core") && mounted(coreModules, "MessageCoordinationDelegate")) {
    reportCalls(violations, `${services}/core/MessageCoordinationDelegate.rs`, ["MemoryLibrary::saveMemoryNow"], "mounted message coordination still invokes the old memory library");
  }
  return violations;
}

/** Rejects mounted chat ownership policy and its actual send/coordination consumers. */
export function chatOwnerResolverViolations() {
  const violations = [];
  if (!mounted(serviceModules, "core")) return violations;
  if (mounted(coreModules, "ChatMemoryOwnerResolver")) {
    reportCalls(violations, `${services}/core/ChatMemoryOwnerResolver.rs`, ["ActivePromptManager::getInstance"], `mounted by ${coreModules}, old chat ownership policy remains`);
  }
  if (mounted(serviceModules, "ChatServiceCore")) {
    reportCalls(violations, `${services}/ChatServiceCore.rs`, ["ChatMemoryOwnerResolver::resolveMemoryOwner"], "chat still invokes old owner resolver");
  }
  if (mounted(coreModules, "MessageCoordinationDelegate")) {
    reportCalls(violations, `${services}/core/MessageCoordinationDelegate.rs`, ["ChatMemoryOwnerResolver::resolveRoleCardId"], "message coordination still invokes old role resolver");
  }
  return violations;
}

/** Rejects the mounted builtin memory executor and its real public/internal tool registrations. */
export function builtinMemoryToolViolations() {
  const violations = [];
  if (!mounted(`${tool}/lib.rs`, "tools")) return violations;
  const modules = `${tool}/tools/mod.rs`, defaults = `${tool}/tools/defaultTool`;
  if (mounted(modules, "defaultTool") && mounted(`${defaults}/mod.rs`, "standard") && mounted(`${defaults}/standard/mod.rs`, "StandardMemoryTools")) {
    reportCalls(violations, `${defaults}/standard/StandardMemoryTools.rs`, ["MemoryRepository::new", "UserMarkdownRepository::new"], "mounted builtin memory executor still owns repository business");
  }
  if (mounted(modules, "ToolRegistration")) {
    const file = `${tool}/tools/ToolRegistration.rs`;
    if (/\bMemoryToolExecutor\s*\{/.test(code(file))) {
      reportCalls(violations, file, ["registerMemoryPublicTools", "registerMemoryInternalTools"], "builtin registration still installs the old memory executor");
    }
  }
  return violations;
}

/** Combines explicit mounted dependency checks without scanning unrelated retained migration references. */
export function mountedBusinessViolations() {
  return [
    ...runtimeManagerViolations(), ...domainTraitViolations(), ...backgroundMemoryViolations(), ...providerChatMemoryViolations(),
    ...chatOwnerResolverViolations(), ...builtinMemoryToolViolations(),
  ];
}
