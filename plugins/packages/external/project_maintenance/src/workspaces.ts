import type { AvailableChat } from "./api.js";

/** Reads the host's real bindings; maintenance never creates its own workspace registry. */
export async function workspaceChats(): Promise<AvailableChat[]> {
  const rows: unknown = JSON.parse(await Tools.SoftwareSettings.exec(["--json", "chat", "list"]));
  if (!Array.isArray(rows)) throw Error("软件返回了无效的工作区聊天列表");
  return rows.map(row => {
    if (!row || typeof row.id !== "string" || typeof row.title !== "string") throw Error("无效的聊天信息");
    return {id: row.id, title: row.title,
      workspaceId: typeof row.workspaceId === "string" ? row.workspaceId : null,
      workspacePath: typeof row.workspacePrimaryPath === "string" ? row.workspacePrimaryPath : null};
  });
}

/** Resolves through the host VFS on the executing device, never by stripping mount prefixes. */
export async function chatWorkspace(chatId: string): Promise<string> {
  const chat = (await workspaceChats()).find(chat => chat.id === chatId);
  if (!chat?.workspaceId || !chat.workspacePath) throw Error("该聊天尚未绑定工作区，请先在软件中绑定工作区。");
  const result = JSON.parse(await Tools.SoftwareSettings.exec([
    "--json", "workspace", "exec", chat.workspacePath, "git", "rev-parse", "--show-toplevel", "--show-prefix",
  ]));
  if (result.exitCode !== 0 || typeof result.stdout !== "string") {
    const detail = typeof result.stderr === "string" ? result.stderr.trim() : "";
    if (/not a git repository/i.test(detail)) throw Error(`工作区「${chat.workspacePath}」尚未初始化为 Git 仓库。请先在该工作区初始化 Git 并完成首次提交，再开启维护；也可以在软件中绑定已有的 Git 仓库根目录。`);
    throw Error(`无法读取工作区「${chat.workspacePath}」的 Git 仓库，请检查目录访问权限和 Git 是否可用。${detail ? "\n" + detail.slice(0, 800) : ""}`);
  }
  const lines = result.stdout.replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
  if (lines.length !== 2 || lines[1] !== "" || !lines[0] || /[\0\r]/.test(lines[0])) throw Error("请将聊天工作区绑定到 Git 仓库根目录。");
  return lines[0];
}
