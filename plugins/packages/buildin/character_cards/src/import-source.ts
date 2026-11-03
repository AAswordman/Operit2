/** Selects pasted JSON or a VFS file path without treating malformed JSON as a path. */
export type ImportSource = { kind: "inline"; content: string } | { kind: "file"; path: string };

/** Classifies one import argument by its leading JSON token; everything else is a file path. */
export function parseImportSource(raw: string): ImportSource {
  const trimmed = raw.trim();
  if (trimmed === "") throw new Error("import requires JSON content or a file path");
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return { kind: "inline", content: trimmed };
  return { kind: "file", path: trimmed };
}

/** Reads a file-backed import through the shared VFS API. */
export async function readImportSource(source: ImportSource): Promise<string> {
  if (source.kind === "inline") return source.content;
  const file = await Tools.Files.read(source.path);
  const content = file.content.replace(/^\uFEFF/, "");
  if (content.trim() === "") throw new Error(`import file is empty: ${source.path}`);
  return content;
}
