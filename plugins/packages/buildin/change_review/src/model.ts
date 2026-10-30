import type { Chat } from "../../../../types/chat";

/** Identifies the exact message revision whose semantic results are being reviewed. */
export interface ReviewMessage {
  timestamp: number;
  variantIndex: number;
  variantCount: number;
  sender: string;
  parts: Chat.MessagePart[];
}

/** Keeps file changes in their original execution order, independently of Git state. */
export interface ReviewFile extends Record<string, ToolPkg.JsonValue> {
  path: string;
  diffContent: string;
  additions: number;
  deletions: number;
}

/** Decodes the explicit XML entity encoding used by persisted tool attributes. */
export function decodeXml(value: string): string {
  return value.replace(/&lt;/g, "<").replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&").replace(/&quot;/g, '"').replace(/&apos;/g, "'");
}

/** Accepts an explicit JSON object, whether carried directly or in an XML attribute. */
export function metadata(value: string | undefined): Record<string, unknown> | null {
  if (value === undefined || value.length === 0) return null;
  const text = /^\s*\{\s*&quot;/.test(value) ? decodeXml(value) : value;
  const decoded: unknown = JSON.parse(text);
  return decoded !== null && typeof decoded === "object" && !Array.isArray(decoded)
    ? decoded as Record<string, unknown> : null;
}

/** Counts the same added and removed lines as the original message summary. */
export function lineStats(diff: string): { additions: number; deletions: number } {
  let additions = 0;
  let deletions = 0;
  for (const line of diff.split("\n")) {
    if (line.startsWith("+") && !line.startsWith("+++")) additions++;
    if (line.startsWith("-") && !line.startsWith("---")) deletions++;
  }
  return { additions, deletions };
}

/** Reads an explicit legacy file-diff envelope, including its CDATA body. */
function envelopeDiffs(text: string): Array<{ path: string; diffContent: string }> {
  const changes: Array<{ path: string; diffContent: string }> = [];
  for (const match of text.matchAll(/<file-diff\b([^>]*)>([\s\S]*?)<\/file-diff>/gi)) {
    const path = /\bpath="([^"]*)"/i.exec(match[1]);
    const cdata = /<!\[CDATA\[([\s\S]*?)\]\]>/i.exec(match[2]);
    changes.push({
      path: path === null ? "" : decodeXml(path[1]),
      diffContent: decodeXml((cdata === null ? match[2] : cdata[1]).trim()),
    });
  }
  return changes;
}

/** Extracts only results explicitly declared with the file-diff renderer. */
export function reviewForMessage(message: ReviewMessage): ReviewFile[] {
  const files = new Map<string, string>();
  for (const part of message.parts) {
    if (part.kind !== "tool_result") continue;
    const display = metadata(part.attributes.display);
    if (display === null || display.resultRenderer !== "fileDiff" ||
        display.label === null || typeof display.label !== "object" ||
        Array.isArray(display.label) ||
        Object.values(display.label).some(value => typeof value !== "string")) continue;
    let changes: Array<{ path: string; diffContent: string }>;
    if (part.attributes.result_data === undefined || part.attributes.result_data === '') {
      changes = envelopeDiffs(part.content);
    } else {
      const data = metadata(part.attributes.result_data);
      const operation = data?.operation;
      if (operation === null || typeof operation !== 'object' || Array.isArray(operation)) throw new Error('File diff requires a typed operation');
      const typed = operation as Record<string, unknown>;
      if (typeof typed.path !== 'string' || typeof data?.diffContent !== 'string' ||
          (typed.details !== undefined && typed.details !== null && typeof typed.details !== 'string')) throw new Error('Invalid typed file diff');
      changes = data.diffContent.length === 0 ? [] : [{ path: typed.path, diffContent: data.diffContent }];
    }
    for (const file of changes) {
      const previous = files.get(file.path);
      files.set(file.path, previous === undefined ? file.diffContent : previous + "\n" + file.diffContent);
    }
  }
  return [...files].map(([path, diffContent]) => ({ path, diffContent, ...lineStats(diffContent) }));
}

/** Returns the original filename and slash-normalized parent shown by the summary. */
export function fileIdentity(path: string): { name: string; directory: string } {
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  return {
    name: index < 0 || index === normalized.length - 1 ? path : normalized.slice(index + 1),
    directory: index < 0 ? "" : normalized.slice(0, index + 1),
  };
}
