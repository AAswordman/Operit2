import type { CapturedMessage } from "./api.js";
export type { Stage, Note, Ticket, CapturedMessage, ChatBinding, AvailableChat, Request } from "./api.js";
export interface CaptureRecord {
  message: CapturedMessage; parentChatId: string; projectPath: string;
  receivedAt: number; ticketId: string | null; ignored?: "acknowledgement" | "status_query" | "status_query_auto";
}
export const CHANNEL = "project_maintenance.service";
export function required(value: unknown, name: string): string {
  if (typeof value !== "string" || !value.trim()) throw new Error(name + " must be non-empty text");
  return value.trim();
}
export function errorText(error: unknown): string { return error instanceof Error ? error.message : String(error); }
