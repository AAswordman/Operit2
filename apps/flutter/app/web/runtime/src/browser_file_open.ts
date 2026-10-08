/** File presentation runs on the browser page, never the OPFS runtime worker. */
const mimeTypes: Record<string, string> = {
  txt: "text/plain", log: "text/plain", md: "text/plain", csv: "text/csv",
  json: "application/json", xml: "text/xml", html: "text/html", htm: "text/html",
  css: "text/plain", js: "text/plain", ts: "text/plain", py: "text/plain", rs: "text/plain",
  pdf: "application/pdf", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg",
  gif: "image/gif", webp: "image/webp", svg: "image/svg+xml", avif: "image/avif",
  bmp: "image/bmp", ico: "image/x-icon", mp3: "audio/mpeg", wav: "audio/wav",
  ogg: "audio/ogg", m4a: "audio/mp4", flac: "audio/flac", mp4: "video/mp4",
  webm: "video/webm", mov: "video/quicktime", zip: "application/zip",
  doc: "application/msword", xls: "application/vnd.ms-excel", ppt: "application/vnd.ms-powerpoint",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
};

export function browserFileMimeType(path: string): string {
  return mimeTypes[path.split("/").pop()!.split(".").pop()!.toLowerCase()] || "application/octet-stream";
}

/** Popup blockers require a real click; cancellation/timeout must not report success. */
function requestFileWindow(name: string): Promise<Window> {
  const opened = window.open("about:blank", "_blank");
  if (opened) return Promise.resolve(opened);
  if (typeof document === "undefined" || typeof HTMLDialogElement === "undefined") {
    return Promise.reject(new Error("The browser blocked opening the file; allow popups and retry"));
  }
  return new Promise((resolve, reject) => {
    const dialog = document.createElement("dialog");
    const label = document.createElement("p");
    label.textContent = `Open file / 打开文件：${name}`;
    const open = document.createElement("button");
    open.textContent = "Open / 打开";
    const cancel = document.createElement("button");
    cancel.textContent = "Cancel / 取消";
    const finish = (popup?: Window, error?: Error): void => {
      clearTimeout(timeout);
      dialog.remove();
      if (popup) resolve(popup); else reject(error || new Error("File opening was cancelled"));
    };
    const timeout = setTimeout(() => finish(undefined, new Error("File opening confirmation timed out")), 55_000);
    open.onclick = () => {
      // Deliberately runs synchronously inside the user's click activation.
      const popup = window.open("about:blank", "_blank");
      finish(popup || undefined, popup ? undefined : new Error("The browser blocked the file window"));
    };
    cancel.onclick = () => finish();
    dialog.oncancel = () => finish();
    dialog.append(label, open, cancel);
    document.body.append(dialog);
    try { dialog.showModal(); } catch (error) { finish(undefined, error as Error); }
  });
}

/** Opens a sandboxed preview with a named download fallback for unsupported formats.
 * Local HTML/SVG must never execute with the application's origin or opener access. */
export async function openBrowserFile(path: string, bytes: Uint8Array): Promise<void> {
  if (!path || path.includes("\0") || !(bytes instanceof Uint8Array)) {
    throw new Error("Invalid browser file-open request");
  }
  if (typeof window === "undefined" || typeof URL.createObjectURL !== "function") {
    throw new Error("Opening a file requires a browser window and Blob URL support");
  }
  const name = path.split("/").pop() || "file";
  const popup = await requestFileWindow(name);
  let url: string | undefined;
  try {
    popup.opener = null;
    url = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: browserFileMimeType(path) }));
    const doc = popup.document;
    doc.title = name;
    const download = doc.createElement("a");
    download.textContent = `Download / 下载：${name}`;
    download.href = url;
    download.download = name;
    download.style.cssText = "display:block;padding:12px;font:14px sans-serif";
    const frame = doc.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts allow-downloads");
    frame.referrerPolicy = "no-referrer";
    frame.title = name;
    frame.src = url;
    frame.style.cssText = "width:100%;height:calc(100vh - 48px);border:0";
    doc.body.style.margin = "0";
    doc.body.replaceChildren(download, frame);
    // Do not revoke immediately: PDF/media viewers may read the URL after initial load.
    const objectUrl = url;
    let released = false;
    const release = (): void => {
      if (released) return;
      released = true;
      URL.revokeObjectURL(objectUrl);
      clearInterval(poll);
      window.removeEventListener("pagehide", release);
    };
    const poll = setInterval(() => { if (popup.closed) release(); }, 1_000);
    popup.addEventListener("pagehide", release, { once: true });
    window.addEventListener("pagehide", release, { once: true });
  } catch (error) {
    if (url) URL.revokeObjectURL(url);
    popup.close();
    throw error;
  }
}
