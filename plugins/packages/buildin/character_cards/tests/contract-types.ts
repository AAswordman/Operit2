import type { Card, DocumentChunk, Group, Memory, MemoryGraph, Store, Tag } from "../src/model";
import type {
  CharacterRecord, GroupRecord, LinkRecord, MemoryGraphRecord, MemoryRecord, StoreRecord, TagRecord,
  DomainInput, DomainOutput,
} from "../src/api";
import type { MemoryLink } from "../src/model";
import type { CharacterRepository } from "../src/canonical";

type Equal<A, B> = [A] extends [B] ? [B] extends [A] ? true : false : false;
type Assert<T extends true> = T;

/** Preserves complete plugin domain records across its editor and public API models. */
export type PluginRecordContracts = [
  Assert<Equal<Card, CharacterRecord>>, Assert<Equal<Group, GroupRecord>>, Assert<Equal<Tag, TagRecord>>,
  Assert<Equal<Store, StoreRecord>>, Assert<Equal<Memory, MemoryRecord>>,
  Assert<Equal<MemoryGraph, MemoryGraphRecord>>, Assert<Equal<MemoryLink, LinkRecord>>,
];

/** Checks provider signatures against the actual generic ToolPkg SDK rather than a dedicated host. */
export function verifyRegistryTypes(
  registry: ToolPkg.Registry,
  command: ToolPkg.CoreCommandHandler,
  api: (event: ToolPkg.PublicApiEvent<DomainInput<"character.create">>) => Promise<DomainOutput<"character.create">>,
): void {
  registry.registerCoreCommand({ id: "test-contract", name: "test-contract", title: "Contract", description: "Type-only provider contract", usage: "test-contract", function: command });
  registry.registerApi<DomainInput<"character.create">, DomainOutput<"character.create">>({ name: "character.create", function: api });
}

/** Pins current file-repository and published API identifier types without requiring legacy numeric compatibility. */
export type FileRepositoryIdentifierContracts = [
  Assert<Equal<Memory["id"], string>>, Assert<Equal<MemoryLink["id"], string>>,
  Assert<Equal<MemoryLink["sourceMemoryId"], Memory["id"]>>, Assert<Equal<MemoryLink["targetMemoryId"], Memory["id"]>>,
  Assert<Equal<DocumentChunk["id"], string>>, Assert<Equal<DocumentChunk["memoryUuid"], Memory["uuid"]>>,
  Assert<Equal<Parameters<CharacterRepository["updateMemory"]>[1], Memory["id"]>>,
  Assert<Equal<Parameters<CharacterRepository["deleteMemory"]>[1], Memory["id"]>>,
  Assert<Equal<Parameters<CharacterRepository["moveMemories"]>[1], Memory["id"][]>>,
  Assert<Equal<Parameters<CharacterRepository["updateLink"]>[1], MemoryLink["id"]>>,
  Assert<Equal<Parameters<CharacterRepository["deleteLink"]>[1], MemoryLink["id"]>>,
  Assert<Equal<DomainInput<"memory.move">["ids"], Memory["id"][]>>,
  Assert<Equal<DomainInput<"memory.link.update">["linkId"], MemoryLink["id"]>>,
];
