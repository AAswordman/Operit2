/** Caches document image references and coalesces overlapping loads by persisted path. */
export function createAvatarSources(
  sources: Map<string, string>,
  resolve: (path: string | null) => Promise<string>,
) {
  const pending = new Map<string, Promise<string>>();
  function source(path: string | null): Promise<string> {
    const key = path ?? "";
    const existing = sources.get(key);
    if (existing !== undefined) return Promise.resolve(existing);
    const active = pending.get(key);
    if (active !== undefined) return active;
    const request = Promise.resolve().then(() => resolve(path)).then(value => {
      sources.set(key, value);
      if (pending.get(key) === request) pending.delete(key);
      return value;
    }, error => {
      if (pending.get(key) === request) pending.delete(key);
      throw error;
    });
    pending.set(key, request);
    return request;
  }
  async function load(paths: readonly (string | null)[]): Promise<void> {
    await Promise.all([...new Set(paths)].map(source));
  }
  return { source, load };
}
