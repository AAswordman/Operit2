import type { Storage } from "../../../../../types/storage";

/** Per-device scheduler cursor file; device-local storage never enters Space synchronization. */
const SCHEDULE_FILE = "memory-schedule.preferences.json";

/**
 * Keeps every memory owner's next automatic check in this device's own plugin data.
 *
 * The value is only meaningful to the device that computed it: another node's clock
 * says nothing about when this device should run. Keeping it out of synchronized
 * records therefore removes a permanently diverging field from memory.sqlite.
 */
export class MemoryScheduleStore {
  /** Retains the device-local preference handle. */
  private constructor(private readonly store: Storage.DataStore) {}

  /** Opens the device-local cursor file, creating it on first use. */
  static async open(): Promise<MemoryScheduleStore> {
    const directory = ToolPkg.getLocalDataDir();
    if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin local data directory is unavailable");
    const target = directory.replace(/[\\/]+$/, "") + "/" + SCHEDULE_FILE;
    return new MemoryScheduleStore(await Tools.Storage.dataStore.open({ path: target }));
  }

  /** Reads one owner's next run on this device's timeline; a missing entry means "immediately due". */
  async read(ownerKey: string): Promise<number> {
    // Always read the published snapshot: another runtime in the same process may have written it.
    const value = (await this.store.data()).get<unknown>(ownerKey);
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : 0;
  }

  /** Records one owner's next run without writing any synchronized record. */
  async write(ownerKey: string, nextRunAtMs: number): Promise<void> {
    if (!Number.isSafeInteger(nextRunAtMs) || nextRunAtMs < 0) throw new Error("Memory schedule time must be a non-negative safe integer");
    await this.store.edit(preferences => { preferences.set(ownerKey, nextRunAtMs); });
  }

  /** Releases the retained device-local handle. */
  async close(): Promise<void> { await this.store.close(); }
}

let opened: Promise<MemoryScheduleStore> | undefined;

/** Shares one device-local cursor handle across scheduler passes and settings reads. */
export function getMemoryScheduleStore(): Promise<MemoryScheduleStore> {
  return opened ??= MemoryScheduleStore.open();
}

/** Releases the shared cursor handle after an explicit plugin shutdown or a test. */
export async function closeMemoryScheduleStore(): Promise<void> {
  const pending = opened;
  opened = undefined;
  if (pending !== undefined) await (await pending).close();
}

/** Reads one owner's next automatic check on this device's timeline. */
export function readNextAutoSaveRun(ownerKey: string): Promise<number> {
  return getMemoryScheduleStore().then(store => store.read(ownerKey));
}

/** Records one owner's next automatic check without touching synchronized records. */
export async function writeNextAutoSaveRun(ownerKey: string, nextRunAtMs: number): Promise<void> {
  await (await getMemoryScheduleStore()).write(ownerKey, nextRunAtMs);
}
