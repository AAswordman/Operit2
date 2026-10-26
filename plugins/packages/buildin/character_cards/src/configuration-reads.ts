import type { ChatConfigurationService } from "./domain";
import type { Card, Group, Store, Tag } from "./model";

/** Reuses an exact record read within one resolution and rejects an inconsistent returned identity. */
function recordRead<T extends { id: string }>(cache: Map<string, Promise<T>>, id: string, read: () => Promise<T>): Promise<T> {
  const existing = cache.get(id);
  if (existing !== undefined) return existing;
  const pending = read().then(
    /** Requires the requested identity before sharing the record with another consumer. */
    record => {
      if (record.id !== id) throw new Error("Configuration read returned a different identity: " + id);
      return record;
    },
  );
  cache.set(id, pending);
  return pending;
}

/** Shares only demanded reads during one configuration request, including an authored opening message. */
export function configurationReadScope(service: ChatConfigurationService): ChatConfigurationService {
  const cards = new Map<string, Promise<Card>>(), groups = new Map<string, Promise<Group>>();
  const tags = new Map<string, Promise<Tag>>(), stores = new Map<string, Promise<Store>>();
  let models: ReturnType<ChatConfigurationService["listModels"]> | undefined;
  let tts: ReturnType<ChatConfigurationService["listTtsConfigs"]> | undefined;
  let preferences: ReturnType<ChatConfigurationService["readGlobalMemoryPreferences"]> | undefined;
  return {
    /** Reads each requested character once. */
    getCharacter: id => recordRead(cards, id, service.getCharacter.bind(service, id)),
    /** Reads each requested group once. */
    getGroup: id => recordRead(groups, id, service.getGroup.bind(service, id)),
    /** Reads each referenced tag once across all group participants. */
    getTag: id => recordRead(tags, id, service.getTag.bind(service, id)),
    /** Reads each referenced shared library once across bindings and mounts. */
    getStore: id => recordRead(stores, id, service.getStore.bind(service, id)),
    /** Loads the model directory once when required. */
    listModels: () => {
      if (models === undefined) models = service.listModels();
      return models;
    },
    /** Loads the speech directory once when required. */
    listTtsConfigs: () => {
      if (tts === undefined) tts = service.listTtsConfigs();
      return tts;
    },
    /** Shares prompt privacy flags while the caller retains its final live privacy check. */
    readGlobalMemoryPreferences: () => {
      if (preferences === undefined) preferences = service.readGlobalMemoryPreferences();
      return preferences;
    },
    /** Preserves typed domain operations on the existing service. */
    dispatchDomain: (operation, input) => service.dispatchDomain(operation, input),
  };
}
