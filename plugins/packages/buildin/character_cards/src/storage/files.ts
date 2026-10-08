import type { CharacterDirectories, CharacterState, UserDocumentWrite } from "../model";
import type { CharacterRepository, CharacterRepositoryOwner } from "../canonical";
import { RepositorySession } from "../repository";
import { assertCharacterState, copyState, createInitialState, decodeCharacterState } from "./state";
import { assertString } from "../validation";

/** Requires the actual successful Files result without replacing failure details. */
function completed(result: { successful: boolean; details: string }): void {
  if (!result.successful) throw new Error(result.details);
}
/** Resolves only the existing package-bound plugin configuration directory. */
function dataDirectory(): string {
  const root = ToolPkg.getConfigDir();
  if (typeof root !== "string" || root.trim() === "") throw new Error("Plugin config directory is unavailable");
  return root.replace(/\\/g, "/").replace(/\/$/, "") + "/character-memory";
}
/** Owns this plugin's file schema and serial operation queue, not a generic runtime storage framework. */
export class FileCharacterRepository implements CharacterRepositoryOwner {
  private busy = false;
  private readonly waiters: (() => void)[] = [];
  private publicationFailure: { error: unknown } | null = null;

  /** Retains only an authoritative successfully opened or published snapshot. */
  private constructor(private readonly directory: string, private state: CharacterState, private readonly directories: CharacterDirectories | null) {}

  /** Initializes a new product installation; existing broken state is never reconstructed. */
  static async open(directories: CharacterDirectories | null = null): Promise<FileCharacterRepository> {
    const directory = dataDirectory(), info = await Tools.Files.exists(directory);
    if (info.exists) {
      if (!info.isDirectory) throw new Error("Plugin data path is not a directory: " + directory);
      const content = await Tools.Files.read(directory + "/state.json"), state = decodeCharacterState(content.content);
      const repository = new FileCharacterRepository(directory, state, directories);
      const pending = await Tools.Files.exists(directory + "/state.next.json");
      if (pending.exists) throw new Error("Unfinished plugin snapshot publication: " + directory + "/state.next.json");
      await repository.verifyDocuments(state);
      return repository;
    }
    completed(await Tools.Files.mkdir(directory, true));
    const state = createInitialState(Date.now()); assertCharacterState(state);
    const repository = new FileCharacterRepository(directory, state, directories);
    const documents = state.owners.map(
      /** Initializes each genuine first-install owner document exactly once. */
      owner => ({ ownerKey: owner.ownerKey, path: directory + "/" + owner.userDocumentPath, content: "", newDocument: true }),
    );
    await repository.publish(state, documents, []);
    return repository;
  }

  /** Runs commands, APIs and UI operations in one private snapshot with one publication phase. */
  async run<T>(action: (repository: CharacterRepository) => Promise<T>): Promise<T> {
    await this.acquire();
    try {
      if (this.publicationFailure !== null) throw this.publicationFailure.error;
      await this.verifyDocuments(this.state);
      const state = copyState(this.state), session = new RepositorySession(state, this.directory, this.directories);
      const result = await action(session); assertCharacterState(state);
      const documents = session.documentWrites();
      const deletedDocuments = this.state.owners.filter(
        /** Identifies removed owners without guessing names or touching other plugin paths. */
        owner => !state.owners.some(
          /** Retains exact owner identities that still exist. */
          retained => retained.ownerKey === owner.ownerKey,
        ),
      ).map(
        /** Selects only this plugin's validated owner USER.md file. */
        owner => this.directory + "/" + owner.userDocumentPath,
      );
      if (JSON.stringify(state) !== JSON.stringify(this.state) || documents.length !== 0) {
        await this.publish(state, documents, deletedDocuments);
        this.state = state;
      }
      return result;
    } finally { this.release(); }
  }

  /** Requires every existing referenced USER.md file and rejects interrupted document publications. */
  private async verifyDocuments(state: CharacterState): Promise<void> {
    for (const owner of state.owners) {
      const path = this.directory + "/" + owner.userDocumentPath;
      const document = await Tools.Files.read(path); assertString(document.content, "USER.md content");
      const pending = await Tools.Files.exists(path + ".next");
      if (pending.exists) throw new Error("Unfinished owner USER.md publication: " + path + ".next");
    }
  }

  /** Acquires the sole publisher without evaluating queued domain work beforehand. */
  private acquire(): Promise<void> {
    if (!this.busy) { this.busy = true; return Promise.resolve(); }
    return new Promise<void>(
      /** Stores the next exact lock grant. */
      resolve => { this.waiters.push(resolve); },
    );
  }
  /** Releases the next caller after its predecessor's original success or failure. */
  private release(): void {
    const waiter = this.waiters.shift();
    if (waiter === undefined) this.busy = false;
    else waiter();
  }
  /** Verifies staging bytes before invoking the existing Files move operation. */
  private async stage(path: string, content: string): Promise<void> {
    completed(await Tools.Files.write(path, content, false));
    const result = await Tools.Files.read(path);
    if (result.content !== content) throw new Error("Plugin staging verification failed: " + path);
  }
  /** Publishes and verifies one real file without promising unsupported host atomicity. */
  private async move(source: string, target: string, content: string): Promise<void> {
    completed(await Tools.Files.move(source, target));
    const result = await Tools.Files.read(target);
    if (result.content !== content) throw new Error("Plugin file publication verification failed: " + target);
  }
  /** Stages all domain/document writes before publication and stops permanently on a publication error. */
  private async publish(state: CharacterState, documents: UserDocumentWrite[], deletedDocuments: string[]): Promise<void> {
    const content = JSON.stringify(state, null, 2) + "\n", pending = this.directory + "/state.next.json", target = this.directory + "/state.json";
    try {
      await this.stage(pending, content);
      for (const document of documents) {
        if (document.newDocument) {
          const existing = await Tools.Files.exists(document.path);
          if (existing.exists) throw new Error("New owner document path is already occupied: " + document.path);
          completed(await Tools.Files.mkdir(document.path.slice(0, document.path.lastIndexOf("/")), true));
        } else {
          const existing = await Tools.Files.read(document.path); assertString(existing.content, "existing USER.md");
        }
        await this.stage(document.path + ".next", document.content);
      }
      for (const document of documents) await this.move(document.path + ".next", document.path, document.content);
      for (const path of deletedDocuments) completed(await Tools.Files.deleteFile(path, false));
      await this.move(pending, target, content);
    } catch (error) {
      this.publicationFailure = { error };
      throw error;
    }
  }
}
