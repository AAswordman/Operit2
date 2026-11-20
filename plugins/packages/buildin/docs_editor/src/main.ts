import screen from "./screen.ui.js";
import { channel, type Command, type DocumentInfo } from "./protocol";

type RegistryRequest =
  | { kind: "register"; info: DocumentInfo }
  | { kind: "unregister"; id: string }
  | { kind: "list"; chatId: string | null }
  | { kind: "command"; chatId: string | null; id: string; command: Command };

const editors = new Map<string, { info: DocumentInfo; contextKey: string }>();

ToolPkg.ipc.on<RegistryRequest, unknown>(channel.registry, async (request, meta) => {
  switch (request.kind) {
    case "register": {
      if (!meta.callerContextKey || !request.info.id) throw new Error("Missing editor context");
      for (const [id, editor] of editors) {
        if (editor.contextKey === meta.callerContextKey) editors.delete(id);
      }
      editors.set(request.info.id, { info: request.info, contextKey: meta.callerContextKey });
      return true;
    }
    case "unregister": {
      if (editors.get(request.id)?.contextKey === meta.callerContextKey) editors.delete(request.id);
      return true;
    }
    case "list": return [...editors.values()].map(item => item.info).filter(info => info.chatId === request.chatId);
    case "command": {
      const editor = editors.get(request.id);
      if (!editor || editor.info.chatId !== request.chatId) throw new Error("Open the document in this conversation's workspace first; then list the open documents.");
      return ToolPkg.ipc.call(channel.editor, request.command, { targetRuntime: "ui", targetContextKey: editor.contextKey });
    }
  }
});

export function registerToolPkg(): boolean {
  const route = "toolpkg:com.operit.office:ui:editor";
  const title = { zh: "Office 文档", en: "Office documents" };
  ToolPkg.registerUiRoute({ id: "editor", route, screen, runtime: "compose_dsl", keepAlive: true, title });
  ToolPkg.registerNavigationEntry({ id: "office_files", surface: "workspace_file", route, title, icon: "Description", order: 100, params: { extensions: ["docx", "pptx", "pdf"] } });
  return true;
}
