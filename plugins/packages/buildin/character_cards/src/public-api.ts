import type { MemoryQueryRequest, MemoryQueryResult, ChatConfigurationRequest, ChatConfigurationResult, DomainInput, DomainOutput } from "./api";
import { parseDomainPayload, parseMemoryQueryRequest } from "./domain";
import { MemoryTools } from "./runtime-tools/memory";
import { dispatchDomain } from "./service-runtime";
export { onMemoryMessagePersisted, onMemoryInterval, registerMemoryJobHooks } from "./memory-jobs/hooks";

const memoryQueries = new MemoryTools(dispatchDomain);

/** Resolves this chat's persisted configuration through the sole initialized service. */
export async function chatConfigurationResolveApi(event: ToolPkg.PublicApiEvent<ChatConfigurationRequest>): Promise<ChatConfigurationResult> {
  if (event.callerPackage !== "host") throw new Error("chat.configuration.resolve requires an authenticated host caller");
  return dispatchDomain("chat.configuration.resolve", parseDomainPayload("chat.configuration.resolve", event.payload));
}

/** Reads the actual binding without changing the current global selection. */
export async function chatConfigurationBindingReadApi(event: ToolPkg.PublicApiEvent<DomainInput<"chat.configuration.binding.read">>): Promise<DomainOutput<"chat.configuration.binding.read">> {
  return dispatchDomain("chat.configuration.binding.read", parseDomainPayload("chat.configuration.binding.read", event.payload));
}

/** Commits the selector's explicit submitted selection before returning it. */
export async function chatConfigurationBindingWriteApi(event: ToolPkg.PublicApiEvent<DomainInput<"chat.configuration.binding.write">>): Promise<DomainOutput<"chat.configuration.binding.write">> {
  return dispatchDomain("chat.configuration.binding.write", parseDomainPayload("chat.configuration.binding.write", event.payload));
}

/** Deletes an existing binding through the same authoritative file service. */
export async function chatConfigurationBindingDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"chat.configuration.binding.delete">>): Promise<DomainOutput<"chat.configuration.binding.delete">> {
  return dispatchDomain("chat.configuration.binding.delete", parseDomainPayload("chat.configuration.binding.delete", event.payload));
}

/** Registers chat APIs without opening storage or evaluating business reads. */
export function registerChatConfigurationApis(): void {
  ToolPkg.registerApi({ name: "chat.configuration.resolve", function: chatConfigurationResolveApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.read", function: chatConfigurationBindingReadApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.write", function: chatConfigurationBindingWriteApi });
  ToolPkg.registerApi({ name: "chat.configuration.binding.delete", function: chatConfigurationBindingDeleteApi });
}

/** Executes participant-bound memory queries through the provider's shared repository and query state. */
export async function memoryQueryApi(event: ToolPkg.PublicApiEvent<MemoryQueryRequest>): Promise<MemoryQueryResult> {
  const input = parseMemoryQueryRequest(event.payload);
  return memoryQueries.execute("query_memory", {
    query: input.query,
    limit: input.limit,
    ...(input.snapshotId === null ? {} : { snapshot_id: input.snapshotId }),
  }, { chatId: null, participantId: input.participantId, callerName: event.callerPackage });
}

/** Executes memory.chat.list through the sole private database-backed service. */
export async function memoryChatListApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.chat.list">>): Promise<DomainOutput<"memory.chat.list">> {
  return dispatchDomain("memory.chat.list", parseDomainPayload("memory.chat.list", event.payload));
}

/** Executes memory.chat.update through the sole private database-backed service. */
export async function memoryChatUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.chat.update">>): Promise<DomainOutput<"memory.chat.update">> {
  return dispatchDomain("memory.chat.update", parseDomainPayload("memory.chat.update", event.payload));
}

/** Executes memory.categorize through the sole private database-backed service. */
export async function memoryCategorizeApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.categorize">>): Promise<DomainOutput<"memory.categorize">> {
  return dispatchDomain("memory.categorize", parseDomainPayload("memory.categorize", event.payload));
}

/** Executes memory.rebuild.start through the sole private database-backed service. */
export async function memoryRebuildStartApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.rebuild.start">>): Promise<DomainOutput<"memory.rebuild.start">> {
  return dispatchDomain("memory.rebuild.start", parseDomainPayload("memory.rebuild.start", event.payload));
}

/** Executes memory.rebuild.progress through the sole private database-backed service. */
export async function memoryRebuildProgressApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.rebuild.progress">>): Promise<DomainOutput<"memory.rebuild.progress">> {
  return dispatchDomain("memory.rebuild.progress", parseDomainPayload("memory.rebuild.progress", event.payload));
}

/** Executes memory.rebuild.cancel through the sole private database-backed service. */
export async function memoryRebuildCancelApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.rebuild.cancel">>): Promise<DomainOutput<"memory.rebuild.cancel">> {
  return dispatchDomain("memory.rebuild.cancel", parseDomainPayload("memory.rebuild.cancel", event.payload));
}

/** Executes memory.embeddings.rebuild through the sole private database-backed service. */
export async function memoryEmbeddingsRebuildApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.embeddings.rebuild">>): Promise<DomainOutput<"memory.embeddings.rebuild">> {
  return dispatchDomain("memory.embeddings.rebuild", parseDomainPayload("memory.embeddings.rebuild", event.payload));
}

/** Executes memory.candidate.enqueue through the sole private database-backed service. */
export async function memoryCandidateEnqueueApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.candidate.enqueue">>): Promise<DomainOutput<"memory.candidate.enqueue">> {
  return dispatchDomain("memory.candidate.enqueue", parseDomainPayload("memory.candidate.enqueue", event.payload));
}

/** Executes full-filter search through the sole file service and its real embedding cache. */
export async function memorySearchWithOptionsApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.searchWithOptions">>): Promise<DomainOutput<"memory.searchWithOptions">> {
  return dispatchDomain("memory.searchWithOptions", parseDomainPayload("memory.searchWithOptions", event.payload));
}

/** Validates and executes the snapshot public domain method. */
export async function snapshotApi(event: ToolPkg.PublicApiEvent<DomainInput<"snapshot">>): Promise<DomainOutput<"snapshot">> {
  return dispatchDomain("snapshot", parseDomainPayload("snapshot", event.payload));
}

/** Validates and executes the character.list public domain method. */
export async function characterListApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.list">>): Promise<DomainOutput<"character.list">> {
  return dispatchDomain("character.list", parseDomainPayload("character.list", event.payload));
}

/** Validates and executes the character.get public domain method. */
export async function characterGetApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.get">>): Promise<DomainOutput<"character.get">> {
  return dispatchDomain("character.get", parseDomainPayload("character.get", event.payload));
}

/** Validates and executes the character.create public domain method. */
export async function characterCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.create">>): Promise<DomainOutput<"character.create">> {
  return dispatchDomain("character.create", parseDomainPayload("character.create", event.payload));
}

/** Validates and executes the character.update public domain method. */
export async function characterUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.update">>): Promise<DomainOutput<"character.update">> {
  return dispatchDomain("character.update", parseDomainPayload("character.update", event.payload));
}

/** Validates and executes the character.delete public domain method. */
export async function characterDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.delete">>): Promise<DomainOutput<"character.delete">> {
  return dispatchDomain("character.delete", parseDomainPayload("character.delete", event.payload));
}

/** Validates and executes the character.setActive public domain method. */
export async function characterSetActiveApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.setActive">>): Promise<DomainOutput<"character.setActive">> {
  return dispatchDomain("character.setActive", parseDomainPayload("character.setActive", event.payload));
}

/** Validates and executes the character.combine public domain method. */
export async function characterCombineApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.combine">>): Promise<DomainOutput<"character.combine">> {
  return dispatchDomain("character.combine", parseDomainPayload("character.combine", event.payload));
}

/** Validates and executes the character.resetDefault public domain method. */
export async function characterResetDefaultApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.resetDefault">>): Promise<DomainOutput<"character.resetDefault">> {
  return dispatchDomain("character.resetDefault", parseDomainPayload("character.resetDefault", event.payload));
}

/** Validates and executes the character.export public domain method. */
export async function characterExportApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.export">>): Promise<DomainOutput<"character.export">> {
  return dispatchDomain("character.export", parseDomainPayload("character.export", event.payload));
}

/** Validates and executes the character.import public domain method. */
export async function characterImportApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.import">>): Promise<DomainOutput<"character.import">> {
  return dispatchDomain("character.import", parseDomainPayload("character.import", event.payload));
}

/** Validates and executes the character.exportBackup public domain method. */
export async function characterExportBackupApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.exportBackup">>): Promise<DomainOutput<"character.exportBackup">> {
  return dispatchDomain("character.exportBackup", parseDomainPayload("character.exportBackup", event.payload));
}

/** Validates and executes the character.importBackup public domain method. */
export async function characterImportBackupApi(event: ToolPkg.PublicApiEvent<DomainInput<"character.importBackup">>): Promise<DomainOutput<"character.importBackup">> {
  return dispatchDomain("character.importBackup", parseDomainPayload("character.importBackup", event.payload));
}

/** Validates and executes the group.list public domain method. */
export async function groupListApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.list">>): Promise<DomainOutput<"group.list">> {
  return dispatchDomain("group.list", parseDomainPayload("group.list", event.payload));
}

/** Validates and executes the group.get public domain method. */
export async function groupGetApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.get">>): Promise<DomainOutput<"group.get">> {
  return dispatchDomain("group.get", parseDomainPayload("group.get", event.payload));
}

/** Validates and executes the group.create public domain method. */
export async function groupCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.create">>): Promise<DomainOutput<"group.create">> {
  return dispatchDomain("group.create", parseDomainPayload("group.create", event.payload));
}

/** Validates and executes the group.update public domain method. */
export async function groupUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.update">>): Promise<DomainOutput<"group.update">> {
  return dispatchDomain("group.update", parseDomainPayload("group.update", event.payload));
}

/** Validates and executes the group.delete public domain method. */
export async function groupDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.delete">>): Promise<DomainOutput<"group.delete">> {
  return dispatchDomain("group.delete", parseDomainPayload("group.delete", event.payload));
}

/** Validates and executes the group.setActive public domain method. */
export async function groupSetActiveApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.setActive">>): Promise<DomainOutput<"group.setActive">> {
  return dispatchDomain("group.setActive", parseDomainPayload("group.setActive", event.payload));
}

/** Validates and executes the group.duplicate public domain method. */
export async function groupDuplicateApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.duplicate">>): Promise<DomainOutput<"group.duplicate">> {
  return dispatchDomain("group.duplicate", parseDomainPayload("group.duplicate", event.payload));
}

/** Validates and executes the group.export public domain method. */
export async function groupExportApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.export">>): Promise<DomainOutput<"group.export">> {
  return dispatchDomain("group.export", parseDomainPayload("group.export", event.payload));
}

/** Validates and executes the group.import public domain method. */
export async function groupImportApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.import">>): Promise<DomainOutput<"group.import">> {
  return dispatchDomain("group.import", parseDomainPayload("group.import", event.payload));
}

/** Validates and executes the group.exportBackup public domain method. */
export async function groupExportBackupApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.exportBackup">>): Promise<DomainOutput<"group.exportBackup">> {
  return dispatchDomain("group.exportBackup", parseDomainPayload("group.exportBackup", event.payload));
}

/** Validates and executes the group.importBackup public domain method. */
export async function groupImportBackupApi(event: ToolPkg.PublicApiEvent<DomainInput<"group.importBackup">>): Promise<DomainOutput<"group.importBackup">> {
  return dispatchDomain("group.importBackup", parseDomainPayload("group.importBackup", event.payload));
}

/** Validates and executes the activePrompt.get public domain method. */
export async function activePromptGetApi(event: ToolPkg.PublicApiEvent<DomainInput<"activePrompt.get">>): Promise<DomainOutput<"activePrompt.get">> {
  return dispatchDomain("activePrompt.get", parseDomainPayload("activePrompt.get", event.payload));
}

/** Validates and executes the activePrompt.setCard public domain method. */
export async function activePromptSetCardApi(event: ToolPkg.PublicApiEvent<DomainInput<"activePrompt.setCard">>): Promise<DomainOutput<"activePrompt.setCard">> {
  return dispatchDomain("activePrompt.setCard", parseDomainPayload("activePrompt.setCard", event.payload));
}

/** Validates and executes the activePrompt.setGroup public domain method. */
export async function activePromptSetGroupApi(event: ToolPkg.PublicApiEvent<DomainInput<"activePrompt.setGroup">>): Promise<DomainOutput<"activePrompt.setGroup">> {
  return dispatchDomain("activePrompt.setGroup", parseDomainPayload("activePrompt.setGroup", event.payload));
}

/** Validates and executes the activePrompt.activateForChat public domain method. */
export async function activePromptActivateForChatApi(event: ToolPkg.PublicApiEvent<DomainInput<"activePrompt.activateForChat">>): Promise<DomainOutput<"activePrompt.activateForChat">> {
  return dispatchDomain("activePrompt.activateForChat", parseDomainPayload("activePrompt.activateForChat", event.payload));
}

/** Validates and executes the activePrompt.resolvedCard public domain method. */
export async function activePromptResolvedCardApi(event: ToolPkg.PublicApiEvent<DomainInput<"activePrompt.resolvedCard">>): Promise<DomainOutput<"activePrompt.resolvedCard">> {
  return dispatchDomain("activePrompt.resolvedCard", parseDomainPayload("activePrompt.resolvedCard", event.payload));
}

/** Validates and executes the tag.list public domain method. */
export async function tagListApi(event: ToolPkg.PublicApiEvent<DomainInput<"tag.list">>): Promise<DomainOutput<"tag.list">> {
  return dispatchDomain("tag.list", parseDomainPayload("tag.list", event.payload));
}

/** Validates and executes the tag.get public domain method. */
export async function tagGetApi(event: ToolPkg.PublicApiEvent<DomainInput<"tag.get">>): Promise<DomainOutput<"tag.get">> {
  return dispatchDomain("tag.get", parseDomainPayload("tag.get", event.payload));
}

/** Validates and executes the tag.create public domain method. */
export async function tagCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"tag.create">>): Promise<DomainOutput<"tag.create">> {
  return dispatchDomain("tag.create", parseDomainPayload("tag.create", event.payload));
}

/** Validates and executes the tag.update public domain method. */
export async function tagUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"tag.update">>): Promise<DomainOutput<"tag.update">> {
  return dispatchDomain("tag.update", parseDomainPayload("tag.update", event.payload));
}

/** Validates and executes the tag.delete public domain method. */
export async function tagDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"tag.delete">>): Promise<DomainOutput<"tag.delete">> {
  return dispatchDomain("tag.delete", parseDomainPayload("tag.delete", event.payload));
}

/** Validates and executes the memory.shared.list public domain method. */
export async function memorySharedListApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.shared.list">>): Promise<DomainOutput<"memory.shared.list">> {
  return dispatchDomain("memory.shared.list", parseDomainPayload("memory.shared.list", event.payload));
}

/** Validates and executes the memory.shared.create public domain method. */
export async function memorySharedCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.shared.create">>): Promise<DomainOutput<"memory.shared.create">> {
  return dispatchDomain("memory.shared.create", parseDomainPayload("memory.shared.create", event.payload));
}

/** Validates and executes the memory.shared.rename public domain method. */
export async function memorySharedRenameApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.shared.rename">>): Promise<DomainOutput<"memory.shared.rename">> {
  return dispatchDomain("memory.shared.rename", parseDomainPayload("memory.shared.rename", event.payload));
}

/** Validates and executes the memory.shared.delete public domain method. */
export async function memorySharedDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.shared.delete">>): Promise<DomainOutput<"memory.shared.delete">> {
  return dispatchDomain("memory.shared.delete", parseDomainPayload("memory.shared.delete", event.payload));
}

/** Validates and executes the memory.mount public domain method. */
export async function memoryMountApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.mount">>): Promise<DomainOutput<"memory.mount">> {
  return dispatchDomain("memory.mount", parseDomainPayload("memory.mount", event.payload));
}

/** Validates and executes the memory.unmount public domain method. */
export async function memoryUnmountApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.unmount">>): Promise<DomainOutput<"memory.unmount">> {
  return dispatchDomain("memory.unmount", parseDomainPayload("memory.unmount", event.payload));
}

/** Validates and executes the memory.user.read public domain method. */
export async function memoryUserReadApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.user.read">>): Promise<DomainOutput<"memory.user.read">> {
  return dispatchDomain("memory.user.read", parseDomainPayload("memory.user.read", event.payload));
}

/** Validates and executes the memory.user.write public domain method. */
export async function memoryUserWriteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.user.write">>): Promise<DomainOutput<"memory.user.write">> {
  return dispatchDomain("memory.user.write", parseDomainPayload("memory.user.write", event.payload));
}

/** Validates and executes the memory.user.path public domain method. */
export async function memoryUserPathApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.user.path">>): Promise<DomainOutput<"memory.user.path">> {
  return dispatchDomain("memory.user.path", parseDomainPayload("memory.user.path", event.payload));
}

/** Validates and executes the memory.resolveOwner public domain method. */
export async function memoryResolveOwnerApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.resolveOwner">>): Promise<DomainOutput<"memory.resolveOwner">> {
  return dispatchDomain("memory.resolveOwner", parseDomainPayload("memory.resolveOwner", event.payload));
}

/** Validates and executes the memory.settings.read public domain method. */
export async function memorySettingsReadApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.settings.read">>): Promise<DomainOutput<"memory.settings.read">> {
  return dispatchDomain("memory.settings.read", parseDomainPayload("memory.settings.read", event.payload));
}

/** Validates and executes the memory.settings.write public domain method. */
export async function memorySettingsWriteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.settings.write">>): Promise<DomainOutput<"memory.settings.write">> {
  return dispatchDomain("memory.settings.write", parseDomainPayload("memory.settings.write", event.payload));
}

/** Validates and executes the memory.searchConfig.read public domain method. */
export async function memorySearchConfigReadApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.searchConfig.read">>): Promise<DomainOutput<"memory.searchConfig.read">> {
  return dispatchDomain("memory.searchConfig.read", parseDomainPayload("memory.searchConfig.read", event.payload));
}

/** Validates and executes the memory.searchConfig.write public domain method. */
export async function memorySearchConfigWriteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.searchConfig.write">>): Promise<DomainOutput<"memory.searchConfig.write">> {
  return dispatchDomain("memory.searchConfig.write", parseDomainPayload("memory.searchConfig.write", event.payload));
}

/** Validates and executes the memory.graph public domain method. */
export async function memoryGraphApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.graph">>): Promise<DomainOutput<"memory.graph">> {
  return dispatchDomain("memory.graph", parseDomainPayload("memory.graph", event.payload));
}

/** Validates and executes the memory.list public domain method. */
export async function memoryListApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.list">>): Promise<DomainOutput<"memory.list">> {
  return dispatchDomain("memory.list", parseDomainPayload("memory.list", event.payload));
}

/** Validates and executes the memory.search public domain method. */
export async function memorySearchApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.search">>): Promise<DomainOutput<"memory.search">> {
  return dispatchDomain("memory.search", parseDomainPayload("memory.search", event.payload));
}

/** Validates and executes the memory.get public domain method. */
export async function memoryGetApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.get">>): Promise<DomainOutput<"memory.get">> {
  return dispatchDomain("memory.get", parseDomainPayload("memory.get", event.payload));
}

/** Validates and executes the memory.create public domain method. */
export async function memoryCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.create">>): Promise<DomainOutput<"memory.create">> {
  return dispatchDomain("memory.create", parseDomainPayload("memory.create", event.payload));
}

/** Validates and executes the memory.update public domain method. */
export async function memoryUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.update">>): Promise<DomainOutput<"memory.update">> {
  return dispatchDomain("memory.update", parseDomainPayload("memory.update", event.payload));
}

/** Validates and deletes the exact string-addressed memory record through the shared service. */
export async function memoryDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.delete">>): Promise<DomainOutput<"memory.delete">> {
  return dispatchDomain("memory.delete", parseDomainPayload("memory.delete", event.payload));
}

/** Validates and moves string-addressed memory records through the shared service. */
export async function memoryMoveApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.move">>): Promise<DomainOutput<"memory.move">> {
  return dispatchDomain("memory.move", parseDomainPayload("memory.move", event.payload));
}

/** Validates and executes the memory.link.create public domain method. */
export async function memoryLinkCreateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.link.create">>): Promise<DomainOutput<"memory.link.create">> {
  return dispatchDomain("memory.link.create", parseDomainPayload("memory.link.create", event.payload));
}

/** Validates and updates the exact string-addressed relationship through the shared service. */
export async function memoryLinkUpdateApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.link.update">>): Promise<DomainOutput<"memory.link.update">> {
  return dispatchDomain("memory.link.update", parseDomainPayload("memory.link.update", event.payload));
}

/** Validates and deletes the exact string-addressed relationship through the shared service. */
export async function memoryLinkDeleteApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.link.delete">>): Promise<DomainOutput<"memory.link.delete">> {
  return dispatchDomain("memory.link.delete", parseDomainPayload("memory.link.delete", event.payload));
}

/** Validates and executes the memory.export public domain method. */
export async function memoryExportApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.export">>): Promise<DomainOutput<"memory.export">> {
  return dispatchDomain("memory.export", parseDomainPayload("memory.export", event.payload));
}

/** Validates and executes the memory.import public domain method. */
export async function memoryImportApi(event: ToolPkg.PublicApiEvent<DomainInput<"memory.import">>): Promise<DomainOutput<"memory.import">> {
  return dispatchDomain("memory.import", parseDomainPayload("memory.import", event.payload));
}

/** Publishes independent typed methods backed by the same service as commands and UI. */
export function registerDomainApis(): void {
  ToolPkg.registerApi<MemoryQueryRequest, MemoryQueryResult>({ name: "memory.query", function: memoryQueryApi });
  ToolPkg.registerApi<DomainInput<"memory.searchWithOptions">, DomainOutput<"memory.searchWithOptions">>({ name: "memory.searchWithOptions", function: memorySearchWithOptionsApi });
  ToolPkg.registerApi<DomainInput<"memory.candidate.enqueue">, DomainOutput<"memory.candidate.enqueue">>({ name: "memory.candidate.enqueue", function: memoryCandidateEnqueueApi });
  ToolPkg.registerApi<DomainInput<"memory.embeddings.rebuild">, DomainOutput<"memory.embeddings.rebuild">>({ name: "memory.embeddings.rebuild", function: memoryEmbeddingsRebuildApi });
  ToolPkg.registerApi<DomainInput<"memory.rebuild.cancel">, DomainOutput<"memory.rebuild.cancel">>({ name: "memory.rebuild.cancel", function: memoryRebuildCancelApi });
  ToolPkg.registerApi<DomainInput<"memory.rebuild.progress">, DomainOutput<"memory.rebuild.progress">>({ name: "memory.rebuild.progress", function: memoryRebuildProgressApi });
  ToolPkg.registerApi<DomainInput<"memory.rebuild.start">, DomainOutput<"memory.rebuild.start">>({ name: "memory.rebuild.start", function: memoryRebuildStartApi });
  ToolPkg.registerApi<DomainInput<"memory.categorize">, DomainOutput<"memory.categorize">>({ name: "memory.categorize", function: memoryCategorizeApi });
  ToolPkg.registerApi<DomainInput<"memory.chat.update">, DomainOutput<"memory.chat.update">>({ name: "memory.chat.update", function: memoryChatUpdateApi });
  ToolPkg.registerApi<DomainInput<"memory.chat.list">, DomainOutput<"memory.chat.list">>({ name: "memory.chat.list", function: memoryChatListApi });
  registerChatConfigurationApis();
  ToolPkg.registerApi<DomainInput<"snapshot">, DomainOutput<"snapshot">>({ name: "snapshot", function: snapshotApi });
  ToolPkg.registerApi<DomainInput<"character.list">, DomainOutput<"character.list">>({ name: "character.list", function: characterListApi });
  ToolPkg.registerApi<DomainInput<"character.get">, DomainOutput<"character.get">>({ name: "character.get", function: characterGetApi });
  ToolPkg.registerApi<DomainInput<"character.create">, DomainOutput<"character.create">>({ name: "character.create", function: characterCreateApi });
  ToolPkg.registerApi<DomainInput<"character.update">, DomainOutput<"character.update">>({ name: "character.update", function: characterUpdateApi });
  ToolPkg.registerApi<DomainInput<"character.delete">, DomainOutput<"character.delete">>({ name: "character.delete", function: characterDeleteApi });
  ToolPkg.registerApi<DomainInput<"character.setActive">, DomainOutput<"character.setActive">>({ name: "character.setActive", function: characterSetActiveApi });
  ToolPkg.registerApi<DomainInput<"character.combine">, DomainOutput<"character.combine">>({ name: "character.combine", function: characterCombineApi });
  ToolPkg.registerApi<DomainInput<"character.resetDefault">, DomainOutput<"character.resetDefault">>({ name: "character.resetDefault", function: characterResetDefaultApi });
  ToolPkg.registerApi<DomainInput<"character.export">, DomainOutput<"character.export">>({ name: "character.export", function: characterExportApi });
  ToolPkg.registerApi<DomainInput<"character.import">, DomainOutput<"character.import">>({ name: "character.import", function: characterImportApi });
  ToolPkg.registerApi<DomainInput<"character.exportBackup">, DomainOutput<"character.exportBackup">>({ name: "character.exportBackup", function: characterExportBackupApi });
  ToolPkg.registerApi<DomainInput<"character.importBackup">, DomainOutput<"character.importBackup">>({ name: "character.importBackup", function: characterImportBackupApi });
  ToolPkg.registerApi<DomainInput<"group.list">, DomainOutput<"group.list">>({ name: "group.list", function: groupListApi });
  ToolPkg.registerApi<DomainInput<"group.get">, DomainOutput<"group.get">>({ name: "group.get", function: groupGetApi });
  ToolPkg.registerApi<DomainInput<"group.create">, DomainOutput<"group.create">>({ name: "group.create", function: groupCreateApi });
  ToolPkg.registerApi<DomainInput<"group.update">, DomainOutput<"group.update">>({ name: "group.update", function: groupUpdateApi });
  ToolPkg.registerApi<DomainInput<"group.delete">, DomainOutput<"group.delete">>({ name: "group.delete", function: groupDeleteApi });
  ToolPkg.registerApi<DomainInput<"group.setActive">, DomainOutput<"group.setActive">>({ name: "group.setActive", function: groupSetActiveApi });
  ToolPkg.registerApi<DomainInput<"group.duplicate">, DomainOutput<"group.duplicate">>({ name: "group.duplicate", function: groupDuplicateApi });
  ToolPkg.registerApi<DomainInput<"group.export">, DomainOutput<"group.export">>({ name: "group.export", function: groupExportApi });
  ToolPkg.registerApi<DomainInput<"group.import">, DomainOutput<"group.import">>({ name: "group.import", function: groupImportApi });
  ToolPkg.registerApi<DomainInput<"group.exportBackup">, DomainOutput<"group.exportBackup">>({ name: "group.exportBackup", function: groupExportBackupApi });
  ToolPkg.registerApi<DomainInput<"group.importBackup">, DomainOutput<"group.importBackup">>({ name: "group.importBackup", function: groupImportBackupApi });
  ToolPkg.registerApi<DomainInput<"activePrompt.get">, DomainOutput<"activePrompt.get">>({ name: "activePrompt.get", function: activePromptGetApi });
  ToolPkg.registerApi<DomainInput<"activePrompt.setCard">, DomainOutput<"activePrompt.setCard">>({ name: "activePrompt.setCard", function: activePromptSetCardApi });
  ToolPkg.registerApi<DomainInput<"activePrompt.setGroup">, DomainOutput<"activePrompt.setGroup">>({ name: "activePrompt.setGroup", function: activePromptSetGroupApi });
  ToolPkg.registerApi<DomainInput<"activePrompt.activateForChat">, DomainOutput<"activePrompt.activateForChat">>({ name: "activePrompt.activateForChat", function: activePromptActivateForChatApi });
  ToolPkg.registerApi<DomainInput<"activePrompt.resolvedCard">, DomainOutput<"activePrompt.resolvedCard">>({ name: "activePrompt.resolvedCard", function: activePromptResolvedCardApi });
  ToolPkg.registerApi<DomainInput<"tag.list">, DomainOutput<"tag.list">>({ name: "tag.list", function: tagListApi });
  ToolPkg.registerApi<DomainInput<"tag.get">, DomainOutput<"tag.get">>({ name: "tag.get", function: tagGetApi });
  ToolPkg.registerApi<DomainInput<"tag.create">, DomainOutput<"tag.create">>({ name: "tag.create", function: tagCreateApi });
  ToolPkg.registerApi<DomainInput<"tag.update">, DomainOutput<"tag.update">>({ name: "tag.update", function: tagUpdateApi });
  ToolPkg.registerApi<DomainInput<"tag.delete">, DomainOutput<"tag.delete">>({ name: "tag.delete", function: tagDeleteApi });
  ToolPkg.registerApi<DomainInput<"memory.shared.list">, DomainOutput<"memory.shared.list">>({ name: "memory.shared.list", function: memorySharedListApi });
  ToolPkg.registerApi<DomainInput<"memory.shared.create">, DomainOutput<"memory.shared.create">>({ name: "memory.shared.create", function: memorySharedCreateApi });
  ToolPkg.registerApi<DomainInput<"memory.shared.rename">, DomainOutput<"memory.shared.rename">>({ name: "memory.shared.rename", function: memorySharedRenameApi });
  ToolPkg.registerApi<DomainInput<"memory.shared.delete">, DomainOutput<"memory.shared.delete">>({ name: "memory.shared.delete", function: memorySharedDeleteApi });
  ToolPkg.registerApi<DomainInput<"memory.mount">, DomainOutput<"memory.mount">>({ name: "memory.mount", function: memoryMountApi });
  ToolPkg.registerApi<DomainInput<"memory.unmount">, DomainOutput<"memory.unmount">>({ name: "memory.unmount", function: memoryUnmountApi });
  ToolPkg.registerApi<DomainInput<"memory.user.read">, DomainOutput<"memory.user.read">>({ name: "memory.user.read", function: memoryUserReadApi });
  ToolPkg.registerApi<DomainInput<"memory.user.write">, DomainOutput<"memory.user.write">>({ name: "memory.user.write", function: memoryUserWriteApi });
  ToolPkg.registerApi<DomainInput<"memory.user.path">, DomainOutput<"memory.user.path">>({ name: "memory.user.path", function: memoryUserPathApi });
  ToolPkg.registerApi<DomainInput<"memory.resolveOwner">, DomainOutput<"memory.resolveOwner">>({ name: "memory.resolveOwner", function: memoryResolveOwnerApi });
  ToolPkg.registerApi<DomainInput<"memory.settings.read">, DomainOutput<"memory.settings.read">>({ name: "memory.settings.read", function: memorySettingsReadApi });
  ToolPkg.registerApi<DomainInput<"memory.settings.write">, DomainOutput<"memory.settings.write">>({ name: "memory.settings.write", function: memorySettingsWriteApi });
  ToolPkg.registerApi<DomainInput<"memory.searchConfig.read">, DomainOutput<"memory.searchConfig.read">>({ name: "memory.searchConfig.read", function: memorySearchConfigReadApi });
  ToolPkg.registerApi<DomainInput<"memory.searchConfig.write">, DomainOutput<"memory.searchConfig.write">>({ name: "memory.searchConfig.write", function: memorySearchConfigWriteApi });
  ToolPkg.registerApi<DomainInput<"memory.graph">, DomainOutput<"memory.graph">>({ name: "memory.graph", function: memoryGraphApi });
  ToolPkg.registerApi<DomainInput<"memory.list">, DomainOutput<"memory.list">>({ name: "memory.list", function: memoryListApi });
  ToolPkg.registerApi<DomainInput<"memory.search">, DomainOutput<"memory.search">>({ name: "memory.search", function: memorySearchApi });
  ToolPkg.registerApi<DomainInput<"memory.get">, DomainOutput<"memory.get">>({ name: "memory.get", function: memoryGetApi });
  ToolPkg.registerApi<DomainInput<"memory.create">, DomainOutput<"memory.create">>({ name: "memory.create", function: memoryCreateApi });
  ToolPkg.registerApi<DomainInput<"memory.update">, DomainOutput<"memory.update">>({ name: "memory.update", function: memoryUpdateApi });
  ToolPkg.registerApi<DomainInput<"memory.delete">, DomainOutput<"memory.delete">>({ name: "memory.delete", function: memoryDeleteApi });
  ToolPkg.registerApi<DomainInput<"memory.move">, DomainOutput<"memory.move">>({ name: "memory.move", function: memoryMoveApi });
  ToolPkg.registerApi<DomainInput<"memory.link.create">, DomainOutput<"memory.link.create">>({ name: "memory.link.create", function: memoryLinkCreateApi });
  ToolPkg.registerApi<DomainInput<"memory.link.update">, DomainOutput<"memory.link.update">>({ name: "memory.link.update", function: memoryLinkUpdateApi });
  ToolPkg.registerApi<DomainInput<"memory.link.delete">, DomainOutput<"memory.link.delete">>({ name: "memory.link.delete", function: memoryLinkDeleteApi });
  ToolPkg.registerApi<DomainInput<"memory.export">, DomainOutput<"memory.export">>({ name: "memory.export", function: memoryExportApi });
  ToolPkg.registerApi<DomainInput<"memory.import">, DomainOutput<"memory.import">>({ name: "memory.import", function: memoryImportApi });
}
