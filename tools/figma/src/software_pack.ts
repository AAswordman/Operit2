/** Whole-software archives contain independent runtime captures, not themes. */
import { unzipSync, zipSync, strFromU8, strToU8 } from "fflate";
import {
  parseDocument,
  MAX_FILE_BYTES,
  type DesignDocument,
  type Json,
} from "./bridge.ts";
export const MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;
export const MAX_EXPANDED_BYTES = 512 * 1024 * 1024;
export interface SoftwareEntry {
  id: string;
  name: string;
  group: string;
  file: string;
  source?: Record<string, Json>;
}
export interface SoftwareIndex {
  format: "operit-software-design";
  version: 1;
  name: string;
  captures: SoftwareEntry[];
  [key: string]: unknown;
}
export interface SoftwarePack {
  index: SoftwareIndex;
  documents: Array<{ entry: SoftwareEntry; document: DesignDocument }>;
}
export function readSoftwarePack(bytes: Uint8Array): SoftwarePack {
  if (bytes.length > MAX_ARCHIVE_BYTES)
    throw new Error("软件设计包超过 256 MiB，请按功能分包");
  let size = 0;
  const files = unzipSync(bytes, {
    filter(file) {
      size += file.originalSize;
      if (size > MAX_EXPANDED_BYTES || file.originalSize > MAX_FILE_BYTES)
        throw new Error("软件设计包解压尺寸超出限制");
      return (
        file.name === "index.json" ||
        /^designs\/[a-zA-Z0-9_.-]+\.operit-design\.json$/.test(file.name)
      );
    },
  });
  if (!files["index.json"] || files["index.json"].length > 8 * 1024 * 1024)
    throw new Error("缺少或超大软件界面目录");
  const index = JSON.parse(strFromU8(files["index.json"])) as SoftwareIndex;
  if (
    index.format !== "operit-software-design" ||
    index.version !== 1 ||
    !Array.isArray(index.captures) ||
    !index.captures.length ||
    index.captures.length > 256
  )
    throw new Error("软件界面目录无效，支持 1 至 256 个状态");
  const ids = new Set<string>(),
    paths = new Set<string>();
  const documents = index.captures.map((entry) => {
    if (
      typeof entry.id !== "string" ||
      !entry.id ||
      ids.has(entry.id) ||
      typeof entry.name !== "string" ||
      typeof entry.group !== "string" ||
      typeof entry.file !== "string" ||
      !/^designs\/[a-zA-Z0-9_.-]+\.operit-design\.json$/.test(entry.file) ||
      paths.has(entry.file) ||
      !files[entry.file]
    )
      throw new Error("软件设计包存在重复状态、缺少文件或非法路径");
    ids.add(entry.id);
    paths.add(entry.file);
    return { entry, document: parseDocument(strFromU8(files[entry.file])) };
  });
  return { index, documents };
}
export function writeSoftwarePack(
  index: SoftwareIndex,
  documents: Array<{ entry: SoftwareEntry; document: DesignDocument }>,
): Uint8Array {
  const files: Record<string, Uint8Array> = {};
  const captures = documents.map(({ entry, document }, i) => {
    const file = `designs/scene-${i + 1}.operit-design.json`;
    files[file] = strToU8(JSON.stringify(document));
    return { ...entry, file };
  });
  files["index.json"] = strToU8(
    JSON.stringify({ ...index, captures }, null, 2),
  );
  const bytes = zipSync(files);
  readSoftwarePack(bytes);
  return bytes;
}
