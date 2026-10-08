import type { EditorContext } from "../../bridge/context";
/** Binds the USER.md dialog to the package's sole typed request transport. */
export function createProfileFeature(context: EditorContext) {
  const { pushDialog, request, renderDialogs, topDialog, toast, popDialog } = context;
  /** Opens USER.md using an explicit character or shared owner reference. */
  async function openUser(ownerKey: string, name: string): Promise<void> {
    const dialog = pushDialog({ type: "user", ownerKey, name, content: "", loading: true, loaded: false });
    try {
      const result = await request({ action: "readUser", ownerKey });
      if (typeof result.content !== "string") throw new Error("USER.md 返回格式不正确");
      dialog.content = result.content; dialog.loaded = true; dialog.loading = false; renderDialogs();
    } catch (error) { dialog.loading = false; dialog.error = String(error); renderDialogs(); }
  }
  /** Persists only an explicitly loaded owner profile. */
  async function handleAction(action: string): Promise<void> {
    if (action !== "save-user") throw new Error("Unknown profile action");
    const dialog = topDialog("user");
    if (!dialog.loaded) throw new Error("用户资料尚未读取成功，不能保存");
    await request({ action: "writeUser", ownerKey: dialog.ownerKey, content: dialog.content });
    popDialog(); toast("用户资料已保存");
  }
  return { openUser, names: ["save-user"], handleAction };
}
