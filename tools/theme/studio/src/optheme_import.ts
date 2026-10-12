export interface ImportedThemeAsset {
  path: string;
  bytes: Uint8Array;
  license: string;
  licensePath: string;
  licenseText: string;
}

export interface ImportedThemeMetadata {
  themeId: string;
  version: string;
  displayName: string;
  authorName: string;
  license: string;
  minAppVersion: string;
  description: string;
  themeLicenseText: string;
}

export interface ImportedThemeBundle {
  kind: "optheme" | "json";
  themeJson: string;
  metadata?: ImportedThemeMetadata;
  assets: ImportedThemeAsset[];
}

interface ZipEntry {
  name: string;
  compressionMethod: number;
  compressedBytes: Uint8Array;
  uncompressedSize: number;
}

function readZipEntries(archive: Uint8Array): ZipEntry[] {
  const view = new DataView(archive.buffer, archive.byteOffset, archive.byteLength);
  let eocdOffset = -1;
  const minEocdSearch = Math.max(0, archive.byteLength - 65557);
  for (let i = archive.byteLength - 22; i >= minEocdSearch; i -= 1) {
    if (view.getUint32(i, true) === 0x06054b50) {
      eocdOffset = i;
      break;
    }
  }
  if (eocdOffset < 0) {
    throw new Error("不是有效的 .optheme / ZIP 归档文件（缺少 EOCD 签名）");
  }

  const totalEntries = view.getUint16(eocdOffset + 10, true);
  const centralDirOffset = view.getUint32(eocdOffset + 16, true);
  const decoder = new TextDecoder("utf-8");
  const entries: ZipEntry[] = [];

  let ptr = centralDirOffset;
  for (let idx = 0; idx < totalEntries; idx += 1) {
    if (ptr + 46 > archive.byteLength || view.getUint32(ptr, true) !== 0x02014b50) {
      throw new Error("ZIP 中央目录损坏");
    }
    const compressionMethod = view.getUint16(ptr + 10, true);
    const compressedSize = view.getUint32(ptr + 20, true);
    const uncompressedSize = view.getUint32(ptr + 24, true);
    const nameLen = view.getUint16(ptr + 28, true);
    const extraLen = view.getUint16(ptr + 30, true);
    const commentLen = view.getUint16(ptr + 32, true);
    const localHeaderOffset = view.getUint32(ptr + 42, true);

    const nameBytes = archive.subarray(ptr + 46, ptr + 46 + nameLen);
    const name = decoder.decode(nameBytes);
    ptr += 46 + nameLen + extraLen + commentLen;

    if (name.endsWith("/")) continue;

    if (
      localHeaderOffset + 30 > archive.byteLength ||
      view.getUint32(localHeaderOffset, true) !== 0x04034b50
    ) {
      throw new Error(`ZIP 本地文件头损坏: ${name}`);
    }
    const localNameLen = view.getUint16(localHeaderOffset + 26, true);
    const localExtraLen = view.getUint16(localHeaderOffset + 28, true);
    const dataStart = localHeaderOffset + 30 + localNameLen + localExtraLen;
    const dataEnd = dataStart + compressedSize;
    if (dataEnd > archive.byteLength) {
      throw new Error(`ZIP 条目超出归档边界: ${name}`);
    }
    entries.push({
      name,
      compressionMethod,
      compressedBytes: archive.subarray(dataStart, dataEnd),
      uncompressedSize,
    });
  }
  return entries;
}

async function decompressEntry(entry: ZipEntry): Promise<Uint8Array> {
  if (entry.compressionMethod === 0) {
    return new Uint8Array(entry.compressedBytes);
  }
  if (entry.compressionMethod === 8 && typeof DecompressionStream !== "undefined") {
    const ds = new DecompressionStream("deflate-raw");
    const writer = ds.writable.getWriter();
    await writer.write(Uint8Array.from(entry.compressedBytes));
    await writer.close();
    const response = new Response(ds.readable);
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
  }
  throw new Error(`不支持的 ZIP 压缩方式 (method=${entry.compressionMethod}): ${entry.name}`);
}

export async function parseImportedThemeFile(file: File): Promise<ImportedThemeBundle> {
  const rawBuffer = await file.arrayBuffer();
  const bytes = new Uint8Array(rawBuffer);
  const decoder = new TextDecoder("utf-8");

  const isZip =
    bytes.byteLength >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    bytes[2] === 0x03 &&
    bytes[3] === 0x04;

  if (!isZip && (file.name.toLowerCase().endsWith(".json") || decoder.decode(bytes.subarray(0, 32)).trimStart().startsWith("{"))) {
    const text = decoder.decode(bytes);
    const parsed = JSON.parse(text) as Record<string, unknown>;
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("导入的 JSON 必须是合法的主题声明对象");
    }
    return {
      kind: "json",
      themeJson: JSON.stringify(parsed, null, 2),
      assets: [],
    };
  }

  const entries = readZipEntries(bytes);
  const fileMap = new Map<string, Uint8Array>();
  for (const entry of entries) {
    const cleanName = entry.name.replace(/^\/+/, "");
    fileMap.set(cleanName, await decompressEntry(entry));
  }

  const themeBytes = fileMap.get("theme.json");
  if (themeBytes == null) {
    throw new Error("主题包缺少必选的 theme.json");
  }
  const themeObj = JSON.parse(decoder.decode(themeBytes)) as Record<string, unknown>;
  const themeJson = JSON.stringify(themeObj, null, 2);

  let metadata: ImportedThemeMetadata | undefined;
  const licenseLookup = new Map<string, { license: string; textPath: string }>();

  const manifestBytes = fileMap.get("manifest.json");
  if (manifestBytes != null) {
    const manifest = JSON.parse(decoder.decode(manifestBytes)) as Record<string, unknown>;
    const authorObj = (manifest.author ?? {}) as Record<string, unknown>;
    const themeLicenseBytes = fileMap.get("licenses/theme.txt");
    metadata = {
      themeId: typeof manifest.themeId === "string" ? manifest.themeId : "org.operit.custom-theme",
      version: typeof manifest.version === "string" ? manifest.version : "1.0.0",
      displayName: typeof manifest.displayName === "string" ? manifest.displayName : "Imported Theme",
      authorName: typeof authorObj.name === "string" ? authorObj.name : "Operit Creator",
      license: typeof manifest.license === "string" ? manifest.license : "MIT",
      minAppVersion: typeof manifest.minAppVersion === "string" ? manifest.minAppVersion : "2.0.0-preview.13",
      description: typeof manifest.description === "string" ? manifest.description : "",
      themeLicenseText: themeLicenseBytes != null ? decoder.decode(themeLicenseBytes) : "",
    };

    const assetLicenses = Array.isArray(manifest.assetLicenses) ? manifest.assetLicenses : [];
    for (const item of assetLicenses) {
      if (item != null && typeof item === "object") {
        const rec = item as Record<string, unknown>;
        if (typeof rec.asset === "string") {
          licenseLookup.set(rec.asset, {
            license: typeof rec.license === "string" ? rec.license : "CC0-1.0",
            textPath: typeof rec.text === "string" ? rec.text : "",
          });
        }
      }
    }
  }

  const assets: ImportedThemeAsset[] = [];
  for (const [entryPath, entryBytes] of fileMap.entries()) {
    if (!entryPath.startsWith("assets/")) continue;
    const licInfo = licenseLookup.get(entryPath);
    const defaultLicense = entryPath.endsWith(".ttf") ? "OFL-1.1" : "CC0-1.0";
    const license = licInfo?.license ?? defaultLicense;
    const stem =
      entryPath
        .split("/")
        .at(-1)
        ?.replace(/\.[^.]+$/, "")
        .toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^[-_]+|[-_]+$/g, "") || "asset";
    const licensePath = licInfo?.textPath || `licenses/${stem}.txt`;
    const licenseFileBytes = licInfo?.textPath ? fileMap.get(licInfo.textPath) : undefined;
    const licenseText =
      licenseFileBytes != null
        ? decoder.decode(licenseFileBytes)
        : `Asset: ${entryPath}\nLicensed under ${license}.`;

    assets.push({
      path: entryPath,
      bytes: entryBytes,
      license,
      licensePath,
      licenseText,
    });
  }

  return {
    kind: "optheme",
    themeJson,
    metadata,
    assets,
  };
}
