/** Cross-device single-flight claims for the interval memory jobs. */
import type { Storage } from "../../../../../types/storage";

/**
 * Every device in a Space runs the same interval hook against the same synced
 * owner records. Without coordination, every pass does the full work and all
 * but one lose the final optimistic-concurrency commit after burning the whole
 * budget. A per-owner lease makes the losers skip in milliseconds: the first
 * runtime to win one compare-and-swap owns the owner for this pass, and the
 * others see a fresh foreign claim.
 */
export class MemoryJobLease {
  /** Must exceed the slowest realistic pass (local model inference) so a live holder is never taken over mid-work. */
  private static readonly TTL_MS = 10 * 60 * 1000;
  private static readonly COLLECTION = "memory_job_lease";
  private readonly holder: string;

  constructor(private readonly database: Storage.ObjectDatabase) {
    /** Distinguishes concurrent runtimes only; uniqueness within one process lifetime is sufficient. */
    this.holder = "jobs-" + Date.now().toString(36) + Math.random().toString(36).slice(2, 10);
  }

  /**
   * Takes the owner's claim, or reports false when another live runtime owns it.
   * A lost race against a concurrent acquirer is the expected skip signal; any
   * other storage failure propagates.
   */
  async acquire(ownerKey: string): Promise<boolean> {
    const entry = await this.collection().get(ownerKey);
    const record = leaseRecord(entry?.value);
    const now = Date.now();
    if (record !== null && record.holder === this.holder) return this.claim(ownerKey, entry!.version, now);
    if (record !== null && record.expiresAt > now) return false;
    return this.claim(ownerKey, entry === null ? null : entry.version, now);
  }

  /**
   * Drops this runtime's finished claim. An expired takeover by another runtime
   * is left intact: that runtime legitimately owns the ongoing pass.
   */
  async release(ownerKey: string): Promise<void> {
    const entry = await this.collection().get(ownerKey);
    if (entry === null) return;
    const record = leaseRecord(entry.value);
    if (record === null || record.holder !== this.holder) return;
    try {
      await this.collection().remove(ownerKey, { expectedVersion: entry.version });
    } catch (error) {
      if (!isVersionConflict(error)) throw error;
    }
  }

  /** Releases the SDK handle for this execution. */
  close(): Promise<void> {
    return this.database.close();
  }

  /** Commits one claim through the exact-version precondition; a conflict loses the race. */
  private async claim(ownerKey: string, expectedVersion: string | null, now: number): Promise<boolean> {
    try {
      await this.collection().put(ownerKey, { holder: this.holder, expiresAt: now + MemoryJobLease.TTL_MS }, { expectedVersion });
      return true;
    } catch (error) {
      if (isVersionConflict(error)) return false;
      throw error;
    }
  }

  private collection(): Storage.Collection<{ holder: string; expiresAt: number }> {
    return this.database.collection(MemoryJobLease.COLLECTION);
  }
}

/** Rejects persisted shapes written by other plugin versions instead of trusting unknown fields. */
function leaseRecord(value: unknown): { holder: string; expiresAt: number } | null {
  if (value === null || typeof value !== "object") return null;
  const record = value as { holder?: unknown; expiresAt?: unknown };
  if (typeof record.holder !== "string" || record.holder === "" || typeof record.expiresAt !== "number") return null;
  return { holder: record.holder, expiresAt: record.expiresAt };
}

/** Recognizes the host's optimistic-concurrency rejection across SDK phrasing. */
function isVersionConflict(error: unknown): boolean {
  return error instanceof Error && error.message.startsWith("Storage version conflict");
}

/** Opens the lease database inside this execution's plugin data directory. */
export async function openMemoryJobLease(): Promise<MemoryJobLease> {
  const directory = ToolPkg.getSpaceDataDir();
  if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin data directory is unavailable");
  return new MemoryJobLease(await Tools.Storage.objects.open({ path: directory + "/memory-jobs-lease.sqlite" }));
}
