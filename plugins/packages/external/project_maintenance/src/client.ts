import type { Inspection } from "./inspection.js";
import { CHANNEL, type Request, type Ticket, type ChatBinding, type AvailableChat } from "./model.js";
export async function call<T>(request: Request): Promise<T> { return ToolPkg.ipc.call<Request, T>(CHANNEL, request, { targetRuntime: "main" }); }
export const tickets = {
  createFromBinding: (chatId: string, feedback: string) => call<Ticket>({ action: "create_from_binding", chatId, feedback }),
  chats: () => call<AvailableChat[]>({ action: "available_chats" }),
  bindings: () => call<ChatBinding[]>({ action: "list_bindings" }),
  bind: (chatId: string, enabled: boolean) => call({ action: "bind_chat", chatId, enabled }),
  list: () => call<Ticket[]>({ action: "list" }),
  inspect: (id: string) => call<Inspection>({ action: "inspect", id }),
  get: (id: string) => call<Ticket>({ action: "get", id }),
  start: (id: string) => call<Ticket>({ action: "start", id }),
  note: (id: string, text: string) => call<Ticket>({ action: "add_note", id, text }),
};
