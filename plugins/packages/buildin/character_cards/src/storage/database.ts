import type { Storage } from "../../../../../types/storage";
import type { CharacterDirectories, CharacterState, UserDocumentWrite } from "../model";
import type { CharacterRepository, CharacterRepositoryOwner } from "../canonical";
import { RepositorySession } from "../repository";
import { assertCharacterState, createInitialState } from "./state";
import { clone, collections, equal, records, stateFromRecords } from "./records";
import type { StoredRecord } from "./records";

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
  private constructor(private readonly database: Storage.ObjectDatabase, private readonly directory: string, private readonly directories: CharacterDirectories | null) {}

  /** Initializes a new plugin database or opens the records written by the centralized Core migration. */
  static async open(directories: CharacterDirectories | null = null): Promise<DatabaseCharacterRepository> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin data directory is unavailable");
    const database = await Tools.Storage.objects.open({ path: directory + "/characters.sqlite" });
    const repository = new DatabaseCharacterRepository(database, directory, directories);
    const metadata = await database.collection("meta").get("state");
    if (metadata === null) {
      const rows = await repository.readRecords();
      if (rows.length !== 0) throw new Error("Character database has records without metadata");
      const state = createInitialState(Date.now()); assertCharacterState(state);
      const documents = state.owners.map(owner => ({ ownerKey: owner.ownerKey, path: directory + "/" + owner.userDocumentPath, content: "", newDocument: true }));
      await repository.publish(state, [], documents, [], null, {});
    } else {
      assertCharacterState(stateFromRecords(metadata.value, await repository.readRecords()));
    }
    return repository;
  }

  /** Loads bounded collection pages through the host without reading a serialized collection file. */
  private async readRecords(): Promise<StoredRecord[]> {
    const rows: StoredRecord[] = [];
    for (const collection of collections) {
      let after: string | null = null;
      for (;;) {
        const page = await this.database.collection(collection).list(after, 1000);
        for (const entry of page) rows.push({ collection, ...entry });
        if (page.length < 1000) break;
        after = page[page.length - 1].key;
      }
    }
    return rows;
  }

  /** Refreshes host records for each operation so remote synchronization and snapshot imports remain visible. */
  async run<T>(action: (repository: CharacterRepository) => Promise<T>): Promise<T> {
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      const metadata = await this.database.collection("meta").get("state");
      if (metadata === null) throw new Error("Character database metadata disappeared");
      const rows = await this.readRecords(), checked = await this.database.collection("meta").get("state");
      if (checked === null || checked.version !== metadata.version) throw new Error("Character records changed during the read; operation was not started");
      const before = stateFromRecords(metadata.value, rows); assertCharacterState(before);
      const state = clone(before), session = new RepositorySession(state, this.directory, this.directories);
      const result = await action(session); assertCharacterState(state);
      const documents = session.documentWrites();
      const removed = before.owners.filter(owner => !state.owners.some(retained => retained.ownerKey === owner.ownerKey)).map(owner => this.directory + "/" + owner.userDocumentPath.slice(0, owner.userDocumentPath.lastIndexOf("/")));
      if (!equal(before, state) || documents.length !== 0) {
        try { await this.publish(state, rows, documents, removed, metadata.version, metadata.value as Record<string, Storage.Value>); }
        catch (error) { this.publicationFailure = { error }; throw error; }
      }
      return result;
    } finally { this.release(); }
  }

  /** Serializes domain sessions within this execution while exact host versions guard other executions. */
  private acquire(): Promise<void> {
    if (!this.busy) { this.busy = true; return Promise.resolve(); }
    return new Promise<void>(resolve => { this.waiters.push(resolve); });
  }

  /** Grants the next waiting operation after the current session completes. */
  private release(): void {
    const waiter = this.waiters.shift();
    if (waiter === undefined) this.busy = false;
    else waiter();
  }

  /** Commits only changed domain records and retains the migration provenance in the metadata guard. */
  private async publish(state: CharacterState, before: StoredRecord[], documents: UserDocumentWrite[], removed: string[], metadataVersion: string | null, metadata: Record<string, Storage.Value>): Promise<void> {
    const previous = new Map(before.map(row => [row.collection + "\0" + row.key, row]));
    const mutations: Storage.ObjectMutation[] = [];
    for (const row of records(state)) {
      const key = row.collection + "\0" + row.key, existing = previous.get(key);
      if (existing === undefined || !equal(existing.value, row.value)) mutations.push({ op: "put", collection: row.collection, key: row.key, value: row.value, expectedVersion: existing === undefined ? null : existing.version });
      previous.delete(key);
    }
    for (const row of previous.values()) mutations.push({ op: "delete", collection: row.collection, key: row.key, expectedVersion: row.version });
    mutations.push({ op: "put", collection: "meta", key: "state", value: { ...metadata, version: state.version, nextId: state.nextId, active: state.active as unknown as Storage.Value }, expectedVersion: metadataVersion });
    if (mutations.length > 1000) throw new Error("Character operation exceeds the host transaction limit of 1000 records");
    const stagedDocuments: { path: string; stagedPath: string }[] = [];
    for (const document of documents) {
      if (document.newDocument) {
        const existing = await Tools.Files.exists(document.path);
        if (existing.exists) throw new Error("New owner document path is already occupied: " + document.path);
        completed(await Tools.Files.mkdir(document.path.slice(0, document.path.lastIndexOf("/")), true));
      }
      const stagedPath = document.path + ".pending";
      completed(await Tools.Files.write(stagedPath, document.content, false));
      stagedDocuments.push({ path: document.path, stagedPath });
    }
    await this.database.commit(mutations);
    for (const document of stagedDocuments) completed(await Tools.Files.move(document.stagedPath, document.path));
    for (const path of removed) completed(await Tools.Files.deleteFile(path, true));
  }
}
