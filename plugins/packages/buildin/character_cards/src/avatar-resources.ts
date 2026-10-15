import type { ComposeWebViewResourceDecision, ComposeWebViewResourceRequest } from "../../../../types/compose-dsl";

/** Owns this document's image references; bytes are read by the existing VFS resource host. */
export function createAvatarResources(origin: string) {
  const references = new Map<string, string>();
  const files = new Map<string, string>();
  const remote = new Set<string>();
  let defaultPath: Promise<string> | null = null;
  let next = 0;

  async function source(path: string | null): Promise<string> {
    if (path === null) {
      if (defaultPath === null) {
        const pending = ToolPkg.readResource("character_default_avatar", "operit-avatar.png").then(path => {
          if (path.trim() === "") throw new Error("默认头像资源路径不能为空");
          return path;
        });
        defaultPath = pending;
        pending.catch(() => { if (defaultPath === pending) defaultPath = null; });
      }
      path = await defaultPath;
    }
    if (path.trim() === "") throw new Error("头像资源路径不能为空");
    if (/^data:image\//i.test(path)) return path;
    const existing = references.get(path);
    if (existing !== undefined) return existing;
    const reference = "avatars/" + (++next);
    references.set(path, reference);
    files.set(origin + reference, path);
    if (/^https?:\/\//i.test(path)) remote.add(path);
    return reference;
  }

  function intercept(request: ComposeWebViewResourceRequest): ComposeWebViewResourceDecision | null {
    if (remote.has(request.url) && !request.isMainFrame && (request.method === "GET" || request.method === "HEAD")) return { action: "allow" };
    const path = files.get(request.url);
    if (path === undefined) return null;
    if (request.isMainFrame || request.method !== "GET" && request.method !== "HEAD") return { action: "block" };
    if (remote.has(path)) return { action: "rewrite", url: path };
    return { action: "respond", response: { statusCode: 200, reasonPhrase: "OK", filePath: path } };
  }

  return { source, intercept };
}
