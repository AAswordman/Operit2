export type Format = "docx" | "pptx" | "pdf";
export type FileInput = { path: string; relativePath: string; name: string; chatId: string | null };
export type DocumentInfo = FileInput & { format: Format; id: string };
export type Command = {
  kind: "read" | "edit" | "save";
  section?: string;
  offset?: number;
  revision?: number;
  change?: Record<string, unknown>;
};
export type Reply = { ok: true; value: unknown } | { ok: false; error: string };
export const channel = { registry: "office.registry", editor: "office.editor" };
export const maxBytes = 128 * 1024 * 1024;
export const chunkSize = 256 * 1024;
export const maxBase64 = Math.ceil(maxBytes / 3) * 4;

export function formatOf(path: string): Format {
  const extension = path.split(".").pop()?.toLowerCase();
  if (extension === "docx" || extension === "pptx" || extension === "pdf") return extension;
  throw new Error("Unsupported Office file type");
}

export function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Expected an object");
  return value as Record<string, unknown>;
}

export function natural(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`Invalid ${label}`);
  return value;
}

export function text(value: unknown, label: string, max = 12000): string {
  if (typeof value !== "string" || value.length > max) throw new Error(`Invalid ${label} (maximum ${max} characters)`);
  return value;
}
