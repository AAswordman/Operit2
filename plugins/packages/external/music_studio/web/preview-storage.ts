import type { StudioStore } from '../src/service';

const LEGACY_KEY = 'operit.music.studio.preview.v1';
/** Large multi-track demos exceed WebKit localStorage; IndexedDB commits the catalog atomically. */
export function createPreviewStorage() {
  let database: Promise<IDBDatabase> | undefined;
  const open = (): Promise<IDBDatabase> => {
    if (!database) database = new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('operit.music.studio.preview', 1);
      request.onupgradeneeded = () => request.result.createObjectStore('state');
      request.onerror = () => { database = undefined; reject(request.error ?? new Error('无法打开浏览器工程存储')); };
      request.onsuccess = () => {
        const db = request.result;
        db.onversionchange = () => { db.close(); database = undefined; };
        resolve(db);
      };
    });
    return database;
  };
  const read = async (): Promise<StudioStore | null> => {
    const db = await open();
    const value = await new Promise<StudioStore | undefined>((resolve, reject) => {
      const tx = db.transaction('state', 'readonly'), request = tx.objectStore('state').get('catalog');
      tx.oncomplete = () => resolve(request.result as StudioStore | undefined);
      tx.onabort = () => reject(tx.error ?? new Error('读取浏览器工程失败'));
      tx.onerror = () => reject(tx.error ?? new Error('读取浏览器工程失败'));
    });
    if (value !== undefined) return value;
    // Existing previews keep working; migrate only after a successful validated save.
    const legacy = localStorage.getItem(LEGACY_KEY);
    return legacy ? JSON.parse(legacy) as StudioStore : null;
  };
  const write = async (store: StudioStore): Promise<void> => {
    const db = await open();
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction('state', 'readwrite');
      tx.objectStore('state').put(store, 'catalog');
      tx.oncomplete = () => resolve();
      tx.onabort = () => reject(tx.error ?? new Error('保存浏览器工程失败'));
      tx.onerror = () => reject(tx.error ?? new Error('保存浏览器工程失败'));
    });
    // A failed write never removes the legacy copy or publishes a new service revision.
    try { localStorage.removeItem(LEGACY_KEY); } catch { /* IndexedDB has already committed. */ }
  };
  return { read, write };
}
