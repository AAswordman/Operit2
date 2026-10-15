import type { Storage } from "../../../../../types/storage";
import type {
  CharacterDirectories,
  CharacterState,
  Card,
  Group,
  GlobalMemoryPreferences,
  MemorySettings,
  MemorySearchConfig,
  UserDocument,
  UserDocumentWrite,
} from "../model";
import type {
  CharacterRepository,
  CharacterRepositoryOwner,
} from "../canonical";
import { RepositorySession } from "../repository";
import { assertCharacterState, createInitialState } from "./state";
import {
  clone,
  collections,
  equal,
  records,
  stateFromRecords,
  storedValue,
} from "./records";
import type { StoredRecord } from "./records";
import { RelationalDatabase, decodeScalarProjection } from "./relational";
import type { RelationalMutation } from "./relational";
import {
  assertBoolean,
  assertCard,
  assertGroup,
  assertObject,
  assertMemorySettings,
  assertSearchConfig,
  assertString,
  requireId,
} from "../validation";
import { userDocumentPath } from "./state";

/** Requires the original Files operation result for actual user document writes. */
function completed(result: { successful: boolean; details: string }): void {
  if (!result.successful) throw new Error(result.details);
}

/** Owns record transactions in the persistent synchronized plugin data namespace. */
export class DatabaseCharacterRepository implements CharacterRepositoryOwner {
  private busy = false;
  private publicationFailure: { error: unknown } | null = null;
  private readonly waiters: (() => void)[] = [];

  /** Retains the host database handle and the explicit external directory readers. */
  private constructor(
    private readonly database: RelationalDatabase,
    private readonly directory: string,
    private readonly directories: CharacterDirectories | null,
  ) {}

  /** Initializes a new plugin database or opens the records written by the centralized Core migration. */
  static async open(
    directories: CharacterDirectories | null = null,
  ): Promise<DatabaseCharacterRepository> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "")
      throw new Error("Plugin data directory is unavailable");
    const database = await RelationalDatabase.open();
    const repository = new DatabaseCharacterRepository(
      database,
      directory,
      directories,
    );
    const metadata = await database.get("meta", "state");
    if (metadata === null) {
      const rows = await repository.readRecords();
      if (rows.length !== 0)
        throw new Error("Character database has records without metadata");
      const state = createInitialState(Date.now());
      assertCharacterState(state);
      const documents = state.owners.map((owner) => ({
        ownerKey: owner.ownerKey,
        path: directory + "/" + owner.userDocumentPath,
        content: "",
        newDocument: true,
      }));
      await repository.publish(state, [], documents, [], null, {});
    } else {
      assertCharacterState(
        stateFromRecords(metadata.value, await repository.readRecords()),
      );
    }
    return repository;
  }

  /** Overlaps the fixed collection reads while preserving each collection's ordered pagination. */
  private async readRecords(): Promise<StoredRecord[]> {
    const results = await Promise.allSettled(
      collections.map(async (collection) => {
        const rows: StoredRecord[] = [];
        for (const entry of await this.database.read(collection))
          rows.push({ collection, ...entry });
        return rows;
      }),
    );
    const rows: StoredRecord[] = [];
    // Settle every read before releasing the repository queue, including on
    // failure, so a subsequent mutation cannot race an unfinished read.
    for (const result of results) {
      if (result.status === "rejected") throw result.reason;
      for (const row of result.value) rows.push(row);
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
      const metadata = await this.database.get("meta", "state");
      if (metadata === null)
        throw new Error("Character database metadata disappeared");
      const rows = await this.readRecords(),
        checked = await this.database.get("meta", "state");
      if (checked === null || checked.version !== metadata.version)
        throw new Error(
          "Character records changed during the read; operation was not started",
        );
      assertObject(metadata.value, "character metadata");
      const before = stateFromRecords(metadata.value, rows);
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
            metadata.version,
            metadata.value,
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

  /** Reads only complete role presentation records under the shared operation queue. */
  async readSidebarDirectory(): Promise<{ cards: Card[]; groups: Group[] }> {
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      const metadata = await this.database.get("meta", "state");
      if (metadata === null)
        throw new Error("Character database metadata disappeared");
      assertObject(metadata.value, "character metadata");
      if (metadata.value.migrationStatus === "writing")
        throw new Error("Character migration has not completed");
      const result = await Promise.allSettled([
        this.database.read("cards"),
        this.database.read("groups"),
      ]);
      const cards: Card[] = [],
        groups: Group[] = [];
      for (const read of result)
        if (read.status === "rejected") throw read.reason;
      if (result[0].status === "fulfilled") {
        for (const row of result[0].value) {
          assertCard(row.value);
          cards.push(row.value);
        }
      }
      if (result[1].status === "fulfilled") {
        for (const row of result[1].value) {
          assertGroup(row.value);
          groups.push(row.value);
        }
      }
      const ids = new Set(cards.map((card) => card.id));
      for (const group of groups)
        for (const member of group.members) {
          if (!ids.has(member.characterCardId))
            throw new Error("Missing group member: " + member.characterCardId);
        }
      const checked = await this.database.get("meta", "state");
      if (checked === null || checked.version !== metadata.version)
        throw new Error("Character directory changed during the read");
      return { cards, groups };
    } finally {
      this.release();
    }
  }

  /** Reads memory settings without loading bodies, vectors or job relations. */
  readMemorySettings(ownerKey: string): Promise<MemorySettings> {
    return this.readOwnerMetadata(ownerKey, (row) => {
      const settings = decodeScalarProjection("owners", "settings", row);
      assertMemorySettings(settings);
      return settings;
    });
  }

  /** Reads only the selected owner's search weights and mode. */
  readMemorySearchConfig(ownerKey: string): Promise<MemorySearchConfig> {
    return this.readOwnerMetadata(ownerKey, (row) => {
      const config = decodeScalarProjection("owners", "searchConfig", row);
      assertSearchConfig(config);
      return config;
    });
  }

  /** Reads global behavior flags directly from metadata without loading any owner records. */
  async readGlobalMemoryPreferences(): Promise<GlobalMemoryPreferences> {
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      const entry = await this.database.get("meta", "state");
      if (entry === null)
        throw new Error("Character database metadata disappeared");
      assertObject(entry.value, "character metadata");
      if (entry.value.migrationStatus === "writing")
        throw new Error("Character migration has not completed");
      const flags = entry.value.memoryPreferences;
      assertObject(flags, "global memory preferences");
      assertBoolean(flags.userDescriptionEnabled, "user description enabled");
      assertBoolean(flags.autoUpdateEnabled, "memory auto update enabled");
      return {
        userDescriptionEnabled: flags.userDescriptionEnabled,
        autoUpdateEnabled: flags.autoUpdateEnabled,
      };
    } finally {
      this.release();
    }
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
    row: Record<string, Storage.SqlValue>,
  ): string {
    assertString(row.userDocumentPath, "owner document path");
    if (row.userDocumentPath !== userDocumentPath(ownerKey))
      throw new Error("Invalid owner document path");
    return this.directory + "/" + row.userDocumentPath;
  }

  /** Serializes one indexed owner query with domain mutations and checks migration readiness. */
  private async readOwnerMetadata<T>(
    ownerKey: string,
    action: (row: Record<string, Storage.SqlValue>) => T | Promise<T>,
  ): Promise<T> {
    requireId(ownerKey, "memory owner");
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      const rows = await this.database.sql.query(
        `SELECT o.* FROM "owners" o JOIN "meta" state ON state."_key" = 'state'
        WHERE o."_key" = ? AND (state."migrationStatus__state" = 0 OR state."migrationStatus" = 'complete')
        AND (EXISTS (SELECT 1 FROM "cards" c WHERE 'character:' || c."id" = o."ownerKey")
          OR EXISTS (SELECT 1 FROM "stores" s WHERE 'shared:' || s."id" = o."ownerKey"))`,
        [ownerKey],
      );
      if (rows.length !== 1 || rows[0].ownerKey !== ownerKey) {
        throw new Error(
          "Memory owner is missing or its migration is incomplete: " + ownerKey,
        );
      }
      return await action(rows[0]);
    } finally {
      this.release();
    }
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
    metadataVersion: string | null,
    metadata: Record<string, Storage.Value>,
  ): Promise<void> {
    const previous = new Map(
      before.map((row) => [row.collection + "\0" + row.key, row]),
    );
    const mutations: RelationalMutation[] = [];
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
    mutations.push({
      op: "put",
      collection: "meta",
      key: "state",
      value: {
        ...metadata,
        version: state.version,
        nextId: state.nextId,
        active: "Model" in state.active ? null : storedValue(state.active),
        memoryPreferences: storedValue(state.memoryPreferences),
      },
      expectedVersion: metadataVersion,
    });
    if (mutations.length > 1000)
      throw new Error(
        "Character operation exceeds the host transaction limit of 1000 records",
      );
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
    await this.database.commit(mutations);
    for (const document of stagedDocuments)
      completed(await Tools.Files.move(document.stagedPath, document.path));
    for (const path of removed)
      completed(await Tools.Files.deleteFile(path, true));
  }
}
