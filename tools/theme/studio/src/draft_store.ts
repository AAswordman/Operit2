/** One local-only draft. No theme bytes are uploaded to a server. */
export interface SavedAsset {
  path: string;
  mediaType: "image/png" | "image/jpeg" | "image/svg+xml" | "font/ttf";
  sha256: string;
  bytes: Blob;
}
export interface SavedDraft {
  themeJson: string;
  metadata: Record<string, string>;
  assets: SavedAsset[];
  toolpkg?: { bytes: Blob; templateId: string };
}

/** Opens the local draft database without sending package bytes to a server. */
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("operit2-theme-studio-v1", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("drafts");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/** Completes one IndexedDB draft operation and closes its connection. */
async function transact<T>(mode: IDBTransactionMode, operation: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const database = await open();
  try {
    return await new Promise<T>((resolve, reject) => {
      const transaction = database.transaction("drafts", mode);
      const request = operation(transaction.objectStore("drafts"));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
  } finally {
    database.close();
  }
}

/** Reads the current local visual draft and its original ToolPkg context. */
export async function loadDraft(): Promise<SavedDraft | undefined> {
  return transact<SavedDraft | undefined>("readonly", store => store.get("current"));
}
/** Saves visual changes and original package context in one local transaction. */
export async function saveDraft(draft: SavedDraft): Promise<void> {
  await transact<IDBValidKey>("readwrite", store => store.put(draft, "current"));
}
