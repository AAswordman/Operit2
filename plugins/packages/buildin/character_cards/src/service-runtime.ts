import type { DomainInput, DomainOperation, DomainOutput } from "./api";
import { executeDomain, resolveConfiguration, dispatch as executeRequest, combinePrompts, resolveMemoryOwner, parseOwner } from "./operations";
import type { ChatConfigurationService } from "./domain";
import { parseDomainPayload } from "./domain";
import { listBindableChats } from "./chat-binding-catalog";
import { parseChatSelection } from "./chat-bindings";
import { MemoryJobRunner } from "./memory-jobs/scheduler";
import { listMemoryChats, pruneMissingMemoryChats } from "./memory-jobs/chat";
import type { CharacterRepositoryOwner } from "./canonical";
import type { Request, RequestOutput, Snapshot } from "./model";
import { requireId } from "./validation";
import type { IndependentConfigurationAccess } from "./selection-application";
import { createSelectionApplication, readSelectionReferences, readThemeChoices, domainSelection, editorSelection, isSelectionOperation, isSelectionRequest } from "./selection-application";
import { independentConfigurations } from "./selection-settings";
import { getRepository, recoverRepository } from "./repository-runtime";
import type { InitializationRecoverySource } from "./storage/initialization";
export { connectDirectorySources } from "./repository-runtime";

export interface CharacterCardsService extends ChatConfigurationService {
  /** Activates the opened chat's persisted selection through the shared configuration application queue. */
  activateChatSelection(chatId: string): Promise<void>;
  readGlobalMemoryPreferences(): Promise<
    import("./model").GlobalMemoryPreferences
  >;
  /** Advances genuine persisted jobs using this service's same repository owner. */
  runMemoryJobs(): Promise<{
    owners: number;
    rebuildWindows: number;
    candidateBatches: number;
  }>;
  /** Dispatches an editor or registered domain command through the supplied canonical port. */
  dispatch<R extends Request>(request: R): Promise<RequestOutput<R>>;
  /** Reads independent canonical catalogs without inserting empty substitute collections. */
  snapshot(): Promise<Snapshot>;
  /** Reads only role records, without unrelated model, speech or tool catalogs. */
  sidebarDirectory(): Promise<Pick<Snapshot, "cards" | "groups">>;
  /** Executes an exact typed domain method on the same service dependency. */
  dispatchDomain<K extends DomainOperation>(
    operation: K,
    input: DomainInput<K>,
  ): Promise<DomainOutput<K>>;
}

/** Shares indexed readers and editing transactions across plugin entry points. */
export function createCharacterCardsService(
  owner: CharacterRepositoryOwner,
  configurations: IndependentConfigurationAccess = independentConfigurations,
): CharacterCardsService {
  const jobs = new MemoryJobRunner(owner),
    application = createSelectionApplication(configurations);
  /** Validates refs before any selection mutation and applies configs only after actual file/native acknowledgement. */
  function dispatchRequest<R extends Request>(
    request: R,
  ): Promise<RequestOutput<R>>;
  /** Executes the discriminated editor request while preserving its complete result union. */
  function dispatchRequest(request: Request): Promise<RequestOutput> {
    if (request.action === "listThemeChoices")
      return readThemeChoices(configurations);
    switch (request.action) {
      case "snapshot":
        return owner.readConfigurationSnapshot();
      case "listBindableChats":
        return listBindableChats(owner.chatBindings);
      case "listMemoryChats":
        return listMemoryChats(request.ownerKey, owner);
      case "pruneMissingChatBindings":
        return owner.run((repository) => pruneMissingMemoryChats(request.ownerKey, repository));
      case "readGlobalMemoryPreferences":
        return owner.readGlobalMemoryPreferences();
      case "readMemoryRecallSettings":
        return owner.readMemoryRecallSettings();
      case "readMemorySettings":
        return owner.readMemorySettings(request.ownerKey);
      case "readMemorySearchConfig":
        return owner.readMemorySearchConfig(request.ownerKey);
      case "readUser":
        return owner.readUser(request.ownerKey);
    }
    if (!isSelectionRequest(request))
      return owner.run(
        /** Executes ordinary editor operations against this same serialized snapshot. */
        (repository) => executeRequest(request, repository),
      );
    return application.commit(
      /** Publishes the actual selection once; independent preference writes are not included in this file publication. */
      (validate) =>
        owner.run(
          /** Prevalidates the chosen complete record inside the same immutable snapshot used by its actual mutation. */
          async (repository) => {
            const references = await validate(
              await readSelectionReferences(
                repository,
                editorSelection(request),
              ),
            );
            return { references, value: await executeRequest(request, repository) };
          },
        ),
    );
  }
  const service: CharacterCardsService = {
    /** Applies the exact persisted actor through the same validated operations used by explicit selection. */
    async activateChatSelection(chatId: string): Promise<void> {
      requireId(chatId, "chat id");
      const binding = await owner.chatBindings.findChatBinding(chatId);
      if (binding === null) return;
      const selection = parseChatSelection(binding.selection);
      switch (selection.kind) {
        case "card": await service.dispatchDomain("activePrompt.setCard", { id: selection.id }); break;
        case "group": await service.dispatchDomain("activePrompt.setGroup", { id: selection.id }); break;
        case "model": await service.dispatchDomain("activePrompt.setModel", {}); break;
      }
    },
    /** Runs the private job scheduler without constructing another store or business service. */
    runMemoryJobs: () => jobs.tick(),
    /** Reads only the metadata flags needed by prompt composition. */
    readGlobalMemoryPreferences: () => owner.readGlobalMemoryPreferences(),
    /** Reads only the selected character and its owned relations. */
    getCharacter: (id) => owner.getCharacter(id),
    /** Reads only the selected group and its saved membership. */
    getGroup: (id) => owner.getGroup(id),
    /** Reads only one referenced prompt tag. */
    getTag: (id) => owner.getTag(id),
    /** Reads only one referenced shared memory library. */
    getStore: (id) => owner.getStore(id),
    /** Reads the model directory independently of role and tool catalogs. */
    listModels: () => owner.listModels(),
    /** Reads the speech directory independently of role and tool catalogs. */
    listTtsConfigs: () => owner.listTtsConfigs(),
    /** Preserves action-specific results through the shared service dispatcher. */
    dispatch: dispatchRequest,
    /** Reads configuration records without loading memory data. */
    snapshot: () => owner.readConfigurationSnapshot(),
    /** Reads role presentation without entering the editing queue. */
    sidebarDirectory: () => owner.readSidebarDirectory(),
    /** Routes reads directly and keeps mutations in the editing transaction. */
    dispatchDomain<K extends DomainOperation>(
      operation: K,
      input: DomainInput<K>,
    ): Promise<DomainOutput<K>> {
      switch (operation) {
        case "snapshot":
          return owner.readConfigurationSnapshot() as Promise<DomainOutput<K>>;
        case "chat.configuration.binding.find": {
          const { chatId } = parseDomainPayload("chat.configuration.binding.find", input);
          return owner.chatBindings.findChatBinding(chatId) as Promise<DomainOutput<K>>;
        }
        case "chat.configuration.resolve":
          return resolveConfiguration(parseDomainPayload("chat.configuration.resolve", input), service) as Promise<DomainOutput<K>>;
        case "character.combine": {
          const { id, promptFunctionType = "CHAT", additionalTagIds = [] } = parseDomainPayload("character.combine", input);
          return combinePrompts(id, additionalTagIds, promptFunctionType, owner)
            .then(prompt => ({ id, promptFunctionType, additionalTagIds, prompt })) as Promise<DomainOutput<K>>;
        }
        case "memory.resolveOwner": {
          const { characterId } = parseDomainPayload("memory.resolveOwner", input);
          return resolveMemoryOwner(characterId, owner).then(parseOwner) as Promise<DomainOutput<K>>;
        }
        case "tag.list":
          return owner.listTags() as Promise<DomainOutput<K>>;
        case "memory.shared.list":
          return owner.listStores() as Promise<DomainOutput<K>>;
        case "activePrompt.get":
          return owner.readActive() as Promise<DomainOutput<K>>;
        case "character.list":
          return owner.listCharacters() as Promise<DomainOutput<K>>;
        case "group.list":
          return owner.listGroups() as Promise<DomainOutput<K>>;
        case "character.get": {
          const { id } = parseDomainPayload("character.get", input);
          return owner.getCharacter(id) as Promise<DomainOutput<K>>;
        }
        case "group.get": {
          const { id } = parseDomainPayload("group.get", input);
          return owner.getGroup(id) as Promise<DomainOutput<K>>;
        }
        case "memory.chat.list":
          return listMemoryChats(
            parseDomainPayload("memory.chat.list", input).ownerKey,
            owner,
          ) as Promise<DomainOutput<K>>;
        case "memory.chat.pruneMissing":
          return owner.run((repository) =>
            pruneMissingMemoryChats(
              parseDomainPayload("memory.chat.pruneMissing", input).ownerKey,
              repository,
            )) as Promise<DomainOutput<K>>;
        case "memory.settings.read":
          return owner.readMemorySettings(
            parseDomainPayload("memory.settings.read", input).ownerKey,
          ) as Promise<DomainOutput<K>>;
        case "memory.searchConfig.read":
          return owner.readMemorySearchConfig(
            parseDomainPayload("memory.searchConfig.read", input).ownerKey,
          ) as Promise<DomainOutput<K>>;
        case "memory.user.read":
          return owner.readUser(
            parseDomainPayload("memory.user.read", input).ownerKey,
          ) as Promise<DomainOutput<K>>;
        case "memory.user.path": {
          const { ownerKey } = parseDomainPayload("memory.user.path", input);
          return owner
            .readUserPath(ownerKey)
            .then((path) => ({ ownerKey, path })) as Promise<DomainOutput<K>>;
        }
      }
      if (!isSelectionOperation(operation))
        return owner.run(
          /** Runs non-selection APIs without touching independent current configurations. */
          (repository) => executeDomain(operation, input, repository),
        );
      return application.commit(
        /** Returns a confirmed owner write before applying any independent preferences. */
        (validate) =>
          owner.run(
            /** Validates the actual domain request and all record references before writing the selected actor. */
            async (repository) => {
              const selection = await domainSelection(
                operation,
                input,
                repository,
              );
              const references = await validate(
                await readSelectionReferences(repository, selection),
              );
              return {
                references,
                value: await executeDomain(operation, input, repository),
              };
            },
          ),
      );
    },
  };
  return service;
}

let service: Promise<CharacterCardsService> | undefined;

/** Opens the shared main-runtime service only on first use. */
export function getService(): Promise<CharacterCardsService> {
  return service ??= getRepository().then(owner => createCharacterCardsService(owner));
}

/** Initializes the same service used by first calls when the application lifecycle is dispatched. */
export async function initializeService(): Promise<void> {
  await getService();
}

/** Replaces the failed service promise only for explicit repair or persisted-plan resumption. */
export async function recoverService(source: InitializationRecoverySource | null): Promise<void> {
  const owner = await recoverRepository(source);
  service = Promise.resolve(createCharacterCardsService(owner));
}

/** Dispatches a UI operation only after the retained initialization has succeeded. */
export async function dispatch<R extends Request>(request: R): Promise<RequestOutput<R>> {
  return (await getService()).dispatch(request);
}

/** Executes a typed domain method on the same initialized business service. */
export async function dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>> {
  return (await getService()).dispatchDomain(operation, input);
}

/** Advances genuine memory jobs through the one retained main-runtime business service. */
export async function runMemoryJobs(): Promise<{ owners: number; rebuildWindows: number; candidateBatches: number }> {
  return (await getService()).runMemoryJobs();
}
