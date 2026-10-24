import type { Storage } from "../../../../../types/storage";
import type {
  CharacterDirectories,
  CharacterState,
  Card,
  ActivePrompt,
  Group,
  GlobalMemoryPreferences,
  MemoryRecallSettings,
  MemorySettings,
  MemorySearchConfig,
  Snapshot,
  Store,
  Tag,
  UserDocument,
  UserDocumentWrite,
} from "../model";
import type {
  CharacterRepository,
  CharacterRepositoryOwner,
} from "../canonical";
import { RepositorySession } from "../repository";
import { assertCharacterState, createInitialState, normalizeMemoryRecallSettings } from "./state";
import {
  clone,
  collections,
  equal,
  records,
  stateFromRecords,
  storedValue,
} from "./records";
import type { StoredRecord } from "./records";
import { CharacterStorage, configurationNames, document, documents, recordPrefix } from "./preferences";
import type { ConfigurationSnapshot } from "./preferences";
import type { ObjectMutation } from "./memory";
import {
  assertBoolean,
  assertCard,
  assertGroup,
  assertInteger,
  assertObject,
  assertString,
  assertTag,
  assertMemorySettings,
  assertSearchConfig,
  requireId,
} from "../validation";
import { userDocumentPath } from "./state";

/** Requires the original Files operation result for actual user document writes. */
function completed(result: { successful: boolean; details: string }): void {
  if (!result.successful) throw new Error(result.details);
}

/** Matches the host's per-transaction object-commit ceiling for plugin records. */
const HOST_OBJECT_COMMIT_LIMIT = 1000;

/** Owns record transactions in the persistent synchronized plugin data namespace. */
export class DatabaseCharacterRepository implements CharacterRepositoryOwner {
  private busy = false;
  private publicationFailure: { error: unknown } | null = null;
  private readonly waiters: (() => void)[] = [];

  /** Retains the host database handle and the explicit external directory readers. */
  private constructor(
    private readonly storage: CharacterStorage,
    private readonly directory: string,
    private readonly directories: CharacterDirectories | null,
  ) {}

  /** Opens storage and initializes first-install records without retaining a synchronized domain snapshot. */
  static async open(
    directories: CharacterDirectories | null = null,
  ): Promise<DatabaseCharacterRepository> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "")
      throw new Error("Plugin data directory is unavailable");
    const storage = await CharacterStorage.open();
    const repository = new DatabaseCharacterRepository(storage, directory, directories);
    const existing = (await storage.metadata.data()).get<Storage.Value>("state");
    if (existing === undefined) {
      const state = createInitialState(Date.now());
      assertCharacterState(state);
      for (const name of configurationNames) {
        await storage.configuration[name].edit(preferences => {
          if (preferences.entries().length !== 0) throw new Error("Character initialization found existing configuration: " + name);
          for (const value of state[name]) preferences.set(recordPrefix + value.id, storedValue(value));
        });
      }
      const initialMemory = records(state).filter(row => !configurationNames.some(name => name === row.collection));
      if (initialMemory.length !== 0) await storage.memory.commit(initialMemory.map(row => ({ op: "put", ...row, expectedVersion: null })));
      for (const owner of state.owners) {
        const path = directory + "/" + owner.userDocumentPath;
        completed(await Tools.Files.mkdir(path.slice(0, path.lastIndexOf("/")), true));
        completed(await Tools.Files.write(path, "", false));
      }
      await storage.metadata.edit(preferences => {
        if (preferences.contains("state")) throw new Error("Character initialization changed concurrently");
        preferences.set("state", { version: state.version, active: storedValue(state.active), memoryPreferences: storedValue(state.memoryPreferences), memoryRecall: storedValue(state.memoryRecall) });
      });
    }
    await repository.readMetadata();
    return repository;
  }

  /** Combines complete JSON preferences with independently stored memory records for a domain session. */
  private async readRecords(configuration: ConfigurationSnapshot): Promise<StoredRecord[]> {
    const rows: StoredRecord[] = [];
    for (const name of configurationNames) {
      for (const [key, value] of configuration[name].entries()) {
        if (key.startsWith(recordPrefix)) rows.push({ collection: name, key: key.slice(recordPrefix.length), value, version: null });
      }
    }
    const results = await Promise.allSettled(["owners", "memories", "links", "chunks", "candidates", "embeddings"].map(async collection =>
      (await this.storage.memory.read(collection)).map(entry => ({ collection, ...entry })),
    ));
    for (const result of results) {
      if (result.status === "rejected") throw result.reason;
      rows.push(...result.value);
    }
    return rows;
  }

  /** Refreshes host records for each operation so remote synchronization and snapshot imports remain visible. */
  async run<T>(
    action: (repository: CharacterRepository) => Promise<T>,
  ): Promise<T> {
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      const metadata = (await this.storage.metadata.data()).get<Storage.Value>("state");
      if (metadata === undefined)
        throw new Error("Character database metadata disappeared");
      const configuration = await this.storage.data(), rows = await this.readRecords(configuration);
      assertObject(metadata, "character metadata");
      const before = stateFromRecords(metadata, rows);
      const state = clone(before),
        session = new RepositorySession(
          state,
          this.directory,
          this.directories,
        );
      const result = await action(session);
      assertCharacterState(state);
      const documents = session.documentWrites();
      const removed = before.owners
        .filter(
          (owner) =>
            !state.owners.some(
              (retained) => retained.ownerKey === owner.ownerKey,
            ),
        )
        .map(
          (owner) =>
            this.directory +
            "/" +
            owner.userDocumentPath.slice(
              0,
              owner.userDocumentPath.lastIndexOf("/"),
            ),
        );
      if (!equal(before, state) || documents.length !== 0) {
        try {
          await this.publish(
            state,
            rows,
            documents,
            removed,
            metadata,
          );
        } catch (error) {
          this.publicationFailure = { error };
          throw error;
        }
      }
      return result;
    } finally {
      this.release();
    }
  }

  /** Rejects operations after an incomplete publication in this execution. */
  private requirePublicationReady(): void {
    if (this.publicationFailure !== null) throw this.publicationFailure.error;
  }

  /** Uses each native owner's immutable snapshot without retrying business callbacks. */
  private async readConsistent<T>(read: () => Promise<T>): Promise<T> {
    this.requirePublicationReady();
    return read();
  }

  /** Reads the role directory from native immutable preference snapshots. */
  async readSidebarDirectory(): Promise<Pick<Snapshot, "cards" | "groups">> {
    this.requirePublicationReady();
    const cards = await this.storage.configuration.cards.data(), groups = await this.storage.configuration.groups.data();
    const value = { cards: documents<Card>(cards), groups: documents<Group>(groups) };
    for (const card of value.cards) assertCard(card);
    const ids = new Set(value.cards.map(card => card.id));
    for (const group of value.groups) {
      assertGroup(group);
      for (const member of group.members) if (!ids.has(member.characterCardId)) throw new Error("Missing group member: " + member.characterCardId);
    }
    return value;
  }

  /** Reads the explicitly stored active selection. */
  readActive(): Promise<ActivePrompt> {
    this.requirePublicationReady();
    return this.readMetadata().then(metadata => metadata.active);
  }

  /** Reads configuration records only; memory content and vectors remain in their own queries. */
  readConfigurationSnapshot(): Promise<Snapshot> {
    if (this.directories === null) throw new Error("External directory readers are unavailable");
    const directories = this.directories;
    return this.readConsistent(async () => {
      const [cards, groups, stores, tags, models, ttsConfigs, toolCatalog, metadata] = await Promise.all([
        this.readCharacters(),
        this.readGroups(),
        this.storage.configuration.stores.data().then(preferences => documents<Store>(preferences)),
        this.readTags(),
        Promise.resolve().then(() => directories.listModels()),
        Promise.resolve().then(() => directories.listTtsConfigs()),
        Promise.resolve().then(() => directories.readToolCatalog()),
        this.readMetadata(),
      ]);
      return { cards, groups, stores, tags, models, ttsConfigs, toolCatalog, ...metadata };
    });
  }

  /** Reads complete character JSON documents. */
  listCharacters(): Promise<Card[]> { return this.readConsistent(() => this.readCharacters()); }
  /** Reads complete group JSON documents. */
  listGroups(): Promise<Group[]> { return this.readConsistent(() => this.readGroups()); }
  /** Reads complete shared-library JSON documents. */
  listStores(): Promise<Store[]> {
    return this.readConsistent(async () => documents<Store>(await this.storage.configuration.stores.data()));
  }
  /** Reads complete prompt-tag JSON documents. */
  listTags(): Promise<Tag[]> { return this.readConsistent(() => this.readTags()); }

  /** Validates complete character documents including nested configuration. */
  private async readCharacters(): Promise<Card[]> {
    return documents<Card>(await this.storage.configuration.cards.data()).map(value => {
      assertCard(value);
      return value;
    });
  }

  /** Validates complete group documents including ordered members. */
  private async readGroups(): Promise<Group[]> {
    return documents<Group>(await this.storage.configuration.groups.data()).map(value => {
      assertGroup(value);
      return value;
    });
  }

  /** Validates complete prompt-tag documents. */
  private async readTags(): Promise<Tag[]> {
    return documents<Tag>(await this.storage.configuration.tags.data()).map(value => {
      assertTag(value);
      return value;
    });
  }

  /** Reads one character by its exact document identity. */
  getCharacter(id: string): Promise<Card> {
    requireId(id, "角色卡标识");
    return this.readConsistent(async () => {
      const value = document<Card>(await this.storage.configuration.cards.data(), id);
      if (value === undefined) throw new Error("Character not found: " + id);
      assertCard(value);
      return value;
    });
  }

  /** Reads one group by its exact document identity. */
  getGroup(id: string): Promise<Group> {
    requireId(id, "群组标识");
    return this.readConsistent(async () => {
      const value = document<Group>(await this.storage.configuration.groups.data(), id);
      if (value === undefined) throw new Error("Group not found: " + id);
      assertGroup(value);
      return value;
    });
  }

  /** Reads one attached prompt tag without scanning the tag directory. */
  getTag(id: string): Promise<Tag> {
    requireId(id, "标签标识");
    return this.readConsistent(async () => {
      const value = document<Tag>(await this.storage.configuration.tags.data(), id);
      if (value === undefined) throw new Error("Tag not found: " + id);
      assertTag(value);
      return value;
    });
  }

  /** Reads one referenced shared library without scanning unrelated stores. */
  getStore(id: string): Promise<Store> {
    requireId(id, "共享记忆库标识");
    return this.readConsistent(async () => {
      const value = document<Store>(await this.storage.configuration.stores.data(), id);
      if (value === undefined) throw new Error("Shared memory not found: " + id);
      assertObject(value, "共享记忆库");
      requireId(value.id, "共享记忆库标识");
      assertString(value.name, "共享记忆库名称");
      assertInteger(value.createdAt, "记忆库创建时间", 0);
      assertInteger(value.updatedAt, "记忆库修改时间", 0);
      return value;
    });
  }

  /** Reads the actual model directory only when model validation needs it. */
  listModels(): ReturnType<CharacterDirectories["listModels"]> {
    this.requirePublicationReady();
    if (this.directories === null) throw new Error("External directory readers are unavailable");
    return this.directories.listModels();
  }

  /** Reads the actual speech directory only when speech validation needs it. */
  listTtsConfigs(): ReturnType<CharacterDirectories["listTtsConfigs"]> {
    this.requirePublicationReady();
    if (this.directories === null) throw new Error("External directory readers are unavailable");
    return this.directories.listTtsConfigs();
  }

  /** Reads the canonical active selection and global preferences. */
  private async readMetadata(): Promise<Pick<Snapshot, "active" | "memoryPreferences">> {
    const metadata = (await this.storage.metadata.data()).get<{
      active: Exclude<ActivePrompt, { Model: { id: "chat" } }> | null;
      memoryPreferences: GlobalMemoryPreferences;
      migrationStatus?: string;
    }>("state");
    if (metadata === undefined) throw new Error("Character database metadata disappeared");
    assertObject(metadata, "character metadata");
    if (metadata.migrationStatus === "writing") throw new Error("Character migration has not completed");
    assertObject(metadata.memoryPreferences, "global memory preferences");
    assertBoolean(metadata.memoryPreferences.userDescriptionEnabled, "user description enabled");
    assertBoolean(metadata.memoryPreferences.autoUpdateEnabled, "memory auto update enabled");
    return {
      active: metadata.active ?? { Model: { id: "chat" } },
      memoryPreferences: metadata.memoryPreferences,
    };
  }

  /** Reads memory settings without loading bodies, vectors or job objects. */
  readMemorySettings(ownerKey: string): Promise<MemorySettings> {
    return this.readOwnerMetadata(ownerKey, (row) => {
      const settings = row.settings;
      assertMemorySettings(settings);
      return settings;
    });
  }

  /** Reads only the selected owner's search weights and mode. */
  readMemorySearchConfig(ownerKey: string): Promise<MemorySearchConfig> {
    return this.readOwnerMetadata(ownerKey, (row) => {
      const config = row.searchConfig;
      assertSearchConfig(config);
      return config;
    });
  }

  /** Reads global memory preferences without loading memory bodies. */
  async readGlobalMemoryPreferences(): Promise<GlobalMemoryPreferences> {
    this.requirePublicationReady();
    return (await this.readMetadata()).memoryPreferences;
  }

  /** Reads recall settings; states persisted by the released plugin predate the field. */
  async readMemoryRecallSettings(): Promise<MemoryRecallSettings> {
    this.requirePublicationReady();
    const metadata = (await this.storage.metadata.data()).get<Record<string, Storage.Value>>("state");
    if (metadata === undefined) throw new Error("Character database metadata disappeared");
    return normalizeMemoryRecallSettings(metadata.memoryRecall === undefined ? { enabled: true } : metadata.memoryRecall);
  }

  /** Reads the selected owner's actual USER.md document. */
  readUser(ownerKey: string): Promise<UserDocument> {
    return this.readOwnerMetadata(ownerKey, async (row) => {
      const document = await Tools.Files.read(
        this.ownerDocumentPath(ownerKey, row),
      );
      assertString(document.content, "USER.md content");
      return { ownerKey, content: document.content };
    });
  }

  /** Resolves the canonical document path for an existing owner. */
  readUserPath(ownerKey: string): Promise<string> {
    return this.readOwnerMetadata(ownerKey, (row) =>
      this.ownerDocumentPath(ownerKey, row),
    );
  }

  /** Validates the stored document identity before exposing a filesystem path. */
  private ownerDocumentPath(
    ownerKey: string,
    row: Record<string, Storage.Value>,
  ): string {
    assertString(row.userDocumentPath, "owner document path");
    if (row.userDocumentPath !== userDocumentPath(ownerKey))
      throw new Error("Invalid owner document path");
    return this.directory + "/" + row.userDocumentPath;
  }

  /** Reads one owner directly; file IO never occupies the full-state editing queue. */
  private async readOwnerMetadata<T>(
    ownerKey: string,
    action: (row: Record<string, Storage.Value>) => T | Promise<T>,
  ): Promise<T> {
    requireId(ownerKey, "memory owner");
    if (this.publicationFailure !== null) throw this.publicationFailure.error;
    return this.readConsistent(async () => {
      const row = await this.storage.memory.get("owners", ownerKey);
      if (row === null) throw new Error("Memory owner is missing: " + ownerKey);
      assertObject(row.value, "memory owner metadata");
      if (row.value.ownerKey !== ownerKey) throw new Error("Invalid memory owner identity");
      const separator = ownerKey.indexOf(":");
      const kind = ownerKey.slice(0, separator);
      if (kind !== "character" && kind !== "shared") throw new Error("Invalid memory owner: " + ownerKey);
      return action(row.value);
    });
  }

  /** Serializes domain sessions within this execution while exact host versions guard other executions. */
  private acquire(): Promise<void> {
    if (!this.busy) {
      this.busy = true;
      return Promise.resolve();
    }
    return new Promise<void>((resolve) => {
      this.waiters.push(resolve);
    });
  }

  /** Grants the next waiting operation after the current session completes. */
  private release(): void {
    const waiter = this.waiters.shift();
    if (waiter === undefined) this.busy = false;
    else waiter();
  }

  /** Commits only changed domain records and retains the migration provenance in the metadata guard. */
  private async publish(
    state: CharacterState,
    before: StoredRecord[],
    documents: UserDocumentWrite[],
    removed: string[],
    metadata: Record<string, Storage.Value>,
  ): Promise<void> {
    const previous = new Map(
      before.map((row) => [row.collection + "\0" + row.key, row]),
    );
    const mutations: ObjectMutation[] = [];
    for (const row of records(state)) {
      const key = row.collection + "\0" + row.key,
        existing = previous.get(key);
      if (existing === undefined || !equal(existing.value, row.value))
        mutations.push({
          op: "put",
          collection: row.collection,
          key: row.key,
          value: row.value,
          expectedVersion: existing === undefined ? null : existing.version,
        });
      previous.delete(key);
    }
    for (const row of previous.values())
      mutations.push({
        op: "delete",
        collection: row.collection,
        key: row.key,
        expectedVersion: row.version,
      });
    const memoryMutations = mutations.filter(row => !configurationNames.some(name => name === row.collection));
    // One host object commit accepts at most 1000 mutations, and a large import exceeds that
    // on its own. Sequential batches keep it possible; a failure after an earlier batch leaves
    // those records committed, which a repeated SKIP/UPDATE import resumes.
    const memoryBatches: ObjectMutation[][] = [];
    for (let start = 0; start < memoryMutations.length; start += HOST_OBJECT_COMMIT_LIMIT)
      memoryBatches.push(memoryMutations.slice(start, start + HOST_OBJECT_COMMIT_LIMIT));
    const stagedDocuments: { path: string; stagedPath: string }[] = [];
    for (const document of documents) {
      if (document.newDocument) {
        const existing = await Tools.Files.exists(document.path);
        if (existing.exists)
          throw new Error(
            "New owner document path is already occupied: " + document.path,
          );
        completed(
          await Tools.Files.mkdir(
            document.path.slice(0, document.path.lastIndexOf("/")),
            true,
          ),
        );
      }
      const stagedPath = document.path + ".pending";
      completed(await Tools.Files.write(stagedPath, document.content, false));
      stagedDocuments.push({ path: document.path, stagedPath });
    }
    for (const name of configurationNames) {
      const changes = mutations.filter(row => row.collection === name);
      if (changes.length === 0) continue;
      await this.storage.configuration[name].edit(preferences => {
        for (const change of changes) {
          // Validate only the record used by this business command; native edit owns publication and synchronization.
          const source = before.find(row => row.collection === name && row.key === change.key);
          const current = preferences.get(recordPrefix + change.key);
          if (source === undefined ? current !== undefined : !equal(current, source.value)) throw new Error("Character record changed before edit: " + change.key);
          if (change.op === "put") preferences.set(recordPrefix + change.key, change.value);
          else preferences.remove(recordPrefix + change.key);
        }
      });
    }
    for (const batch of memoryBatches) await this.storage.memory.commit(batch);
    const nextMetadata = { ...metadata, version: state.version, active: "Model" in state.active ? null : storedValue(state.active), memoryPreferences: storedValue(state.memoryPreferences), memoryRecall: storedValue(state.memoryRecall) };
    await this.storage.metadata.edit(preferences => {
      if (!equal(preferences.get("state"), metadata)) throw new Error("Character metadata edit conflict");
      preferences.set("state", nextMetadata);
    });
    for (const document of stagedDocuments)
      completed(await Tools.Files.move(document.stagedPath, document.path));
    for (const path of removed)
      completed(await Tools.Files.deleteFile(path, true));
  }
}
