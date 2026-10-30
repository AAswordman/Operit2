import {
  parseDocument,
  validateDocument,
  type DesignDocument,
  type Theme,
} from "../../../figma/src/bridge.ts";
import { themeDocument } from "../../../figma/src/theme_scene.ts";
import {
  parseImportedThemeFile,
  type ImportedThemeBundle,
  type ImportedToolPkgSource,
} from "./toolpkg_import.ts";

export function encodeBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i += 16384)
    binary += String.fromCharCode(...bytes.subarray(i, i + 16384));
  return btoa(binary);
}
export function decodeBase64(value: string): Uint8Array {
  return Uint8Array.from(atob(value), (c) => c.charCodeAt(0));
}
export function exportFigmaDocument(
  theme: Theme,
  assets: { path: string; mediaType: string; bytes: Uint8Array }[],
  metadata: Record<string, string>,
  source?: ImportedToolPkgSource,
): DesignDocument {
  const doc = themeDocument(
    theme,
    assets.map((a) => ({
      path: a.path,
      mediaType: a.mediaType,
      base64: encodeBase64(a.bytes),
    })),
    metadata["display-name"] || "Operit UI",
  );
  doc.studioContext = {
    metadata,
    ...(source
      ? {
          toolpkg: {
            base64: encodeBase64(source.bytes),
            templateId: source.template.id,
          },
        }
      : {}),
  };
  return validateDocument(doc);
}
export async function importFigmaDocument(
  text: string,
): Promise<{
  bundle: ImportedThemeBundle;
  metadata?: Record<string, string>;
  changes: number;
}> {
  const doc = parseDocument(text);
  if (!doc.theme)
    throw new Error(
      "这是应用／工具／插件的运行时设计交接稿。请用 Figma 导出的 changes 和 source 定位代码修改；它没有可应用的主题字段。",
    );
  let original: ImportedThemeBundle | undefined;
  const source = doc.studioContext?.toolpkg;
  if (source)
    original = await parseImportedThemeFile(
      new File(
        [decodeBase64(source.base64).buffer as ArrayBuffer],
        "original.toolpkg",
      ),
      async () => source.templateId,
    );
  return {
    bundle: {
      kind: original?.kind ?? "json",
      themeJson: JSON.stringify(doc.theme, null, 2),
      assets: doc.assets.map((a) => ({
        path: a.path,
        bytes: decodeBase64(a.base64),
      })),
      metadata: original?.metadata,
      source: original?.source,
    },
    metadata: doc.studioContext?.metadata,
    changes: doc.changes?.length ?? 0,
  };
}
