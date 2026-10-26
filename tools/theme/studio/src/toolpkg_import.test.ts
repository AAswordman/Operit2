import assert from "node:assert/strict";
import test from "node:test";
import { parseImportedThemeFile, updateToolPkgTheme, validateToolPkgId } from "./toolpkg_import.ts";

const encoder = new TextEncoder();

/** Computes the ZIP CRC-32 for fixture entry bytes. */
function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ ((crc & 1) * 0xedb88320);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

/** Builds a Stored ZIP fixture with ordinary ToolPkg manifest and resource entries. */
function archiveFile(manifest: Record<string, unknown>, resources: Record<string, string> = {}, manifestPath = "manifest.json", manifestText = JSON.stringify(manifest)): File {
  const parts: Uint8Array[] = [];
  const directory: Uint8Array[] = [];
  let offset = 0;
  const entries = { [manifestPath]: manifestText, ...resources };
  for (const [path, text] of Object.entries(entries)) {
    const name = encoder.encode(path);
    const bytes = encoder.encode(text);
    const local = new Uint8Array(30 + name.length + bytes.length);
    const localView = new DataView(local.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, 20, true);
    localView.setUint32(14, crc32(bytes), true);
    localView.setUint32(18, bytes.length, true);
    localView.setUint32(22, bytes.length, true);
    localView.setUint16(26, name.length, true);
    local.set(name, 30);
    local.set(bytes, 30 + name.length);
    const central = new Uint8Array(46 + name.length);
    const centralView = new DataView(central.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, 20, true);
    centralView.setUint16(6, 20, true);
    centralView.setUint32(16, crc32(bytes), true);
    centralView.setUint32(20, bytes.length, true);
    centralView.setUint32(24, bytes.length, true);
    centralView.setUint16(28, name.length, true);
    centralView.setUint32(42, offset, true);
    central.set(name, 46);
    parts.push(local);
    directory.push(central);
    offset += local.length;
  }
  const end = new Uint8Array(22);
  const endView = new DataView(end.buffer);
  endView.setUint32(0, 0x06054b50, true);
  endView.setUint16(8, directory.length, true);
  endView.setUint16(10, directory.length, true);
  endView.setUint32(12, directory.reduce((size, entry) => size + entry.length, 0), true);
  endView.setUint32(16, offset, true);
  return new File([...parts, ...directory, end].map(part => Uint8Array.from(part).buffer), "fixture.toolpkg");
}

/** Creates package metadata with theme declarations and independent script capabilities. */
function manifest(themes: unknown[]): Record<string, unknown> {
  return {
    schema_version: 1,
    api_version: "2.0.0",
    toolpkg_id: "org.operit.fixture",
    version: "1.0.0",
    display_name: "ToolPkg fixture",
    author: ["Creator"],
    description: "A package with scripts and theme templates",
    main: "main.js",
    license: "licenses/agreement.txt",
    themes,
  };
}

const resources: Record<string, string> = {
  "main.js": "export function register() {}",
  "templates/night/visual.json": JSON.stringify({ palette: { seed: "#123456" } }),
  "templates/night/cover.png": "preview fixture",
  "templates/day/visual.json": JSON.stringify({ palette: { seed: "#abcdef" } }),
  "templates/day/cover.png": "preview fixture",
  "licenses/agreement.txt": "Full agreement text",
};
const night = { id: "night", path: "templates/night/visual.json", preview: "templates/night/cover.png" };
const day = { id: "day", path: "templates/day/visual.json", preview: "templates/day/cover.png" };

/** Verifies scripts and non-default paths do not create a separate package classification. */
test("reads a theme declaration from an ordinary ToolPkg with main", async () => {
  const result = await parseImportedThemeFile(archiveFile(manifest([night]), resources));
  assert.equal(result.kind, "toolpkg");
  assert.equal(result.metadata?.themeId, "org.operit.fixture");
  assert.equal(result.metadata?.themeLicenseText, "Full agreement text");
  assert.deepEqual(JSON.parse(result.themeJson), { palette: { seed: "#123456" } });
});

/** Verifies multiple declarations require and honor an explicit selection. */
test("selects a declared theme without enforcing one declaration per ToolPkg", async () => {
  const file = archiveFile(manifest([night, day]), resources);
  const result = await parseImportedThemeFile(file, async (themes) => {
    assert.deepEqual(themes.map(theme => theme.id), ["night", "day"]);
    return "day";
  });
  assert.deepEqual(JSON.parse(result.themeJson), { palette: { seed: "#abcdef" } });
  await assert.rejects(parseImportedThemeFile(file), /请指定/);
  await assert.rejects(parseImportedThemeFile(file, async () => "absent"), /不存在 theme 声明/);
});

/** Verifies malformed references fail instead of substituting fixed theme paths. */
test("rejects missing references and duplicate declaration IDs", async () => {
  await assert.rejects(parseImportedThemeFile(archiveFile(manifest([{ ...night, path: "missing.json" }]), resources)), /引用的文件不存在/);
  await assert.rejects(parseImportedThemeFile(archiveFile(manifest([{ ...night, preview: "missing.png" }]), resources)), /引用的文件不存在/);
  await assert.rejects(parseImportedThemeFile(archiveFile(manifest([night, night]), resources)), /ID 必须非空且唯一/);
  await assert.rejects(parseImportedThemeFile(archiveFile(manifest([]), resources)), /没有可供编辑/);
});

/** Verifies the existing visual JSON draft workflow remains supported. */
test("opens a JSON draft directly", async () => {
  const result = await parseImportedThemeFile(new File([resources[night.path]], "visual.json"));
  assert.equal(result.kind, "json");
  assert.deepEqual(JSON.parse(result.themeJson), { palette: { seed: "#123456" } });
  assert.deepEqual(result.assets, []);
});

/** Verifies editor-specific metadata requirements do not reject standard ToolPkg input. */
test("imports ToolPkg declarations without requiring export-form metadata", async () => {
  const result = await parseImportedThemeFile(archiveFile({ toolpkg_id: "minimal", main: "main.js", themes: [night] }, resources));
  assert.equal(result.metadata?.themeId, "minimal");
  assert.equal(result.metadata?.version, undefined);
  assert.equal(result.metadata?.themeLicenseText, undefined);
});

/** Verifies supported ToolPkg metadata shapes are not mistaken for a different package format. */
test("reads localized metadata and string author", async () => {
  const data = manifest([night]);
  data.display_name = { zh: "工具模板", en: "Tool template" };
  data.description = { values: { zh: "描述", en: "Description" } };
  data.author = "Creator";
  const result = await parseImportedThemeFile(archiveFile(data, resources));
  assert.equal(result.metadata?.displayName, "工具模板");
  assert.equal(result.metadata?.description, "描述");
  assert.equal(result.metadata?.authorName, "Creator");
});

/** Verifies the removed archive extension is not silently accepted as ToolPkg. */
test("rejects the removed optheme format", async () => {
  const file = new File([await archiveFile(manifest([night]), resources).arrayBuffer()], "old.optheme");
  await assert.rejects(parseImportedThemeFile(file), /请选择 .toolpkg/);
});

/** Verifies stored entries cannot misdeclare their expanded byte size. */
test("rejects a stored ZIP entry size mismatch", async () => {
  const bytes = new Uint8Array(await archiveFile(manifest([night]), resources).arrayBuffer());
  const view = new DataView(bytes.buffer);
  const endOffset = bytes.length - 22;
  const centralOffset = view.getUint32(endOffset + 16, true);
  view.setUint32(centralOffset + 24, view.getUint32(centralOffset + 24, true) + 1, true);
  await assert.rejects(parseImportedThemeFile(new File([bytes.buffer], "bad.toolpkg")), /entry size mismatch/);
});

/** Keeps unrelated capabilities and a second theme intact when exporting an edited template. */
test("updates a selected template without stripping ordinary ToolPkg capabilities", async () => {
  const original = await parseImportedThemeFile(archiveFile(manifest([night, day]), resources), async () => "night");
  assert.ok(original.source);
  const authoredManifest = manifest([{ id: "default", path: "theme.json", preview: "preview.png", files: [] }]);
  authoredManifest.license = "licenses/agreement.txt";
  const authored = archiveFile(authoredManifest, {
    "theme.json": JSON.stringify({ palette: { seed: "#556677" } }),
    "preview.png": "edited preview", "licenses/agreement.txt": "Edited agreement",
  });
  const bytes = await updateToolPkgTheme(original.source, new Uint8Array(await authored.arrayBuffer()));
  const updated = await parseImportedThemeFile(new File([Uint8Array.from(bytes).buffer], "result.toolpkg"), async () => "night");
  assert.deepEqual(JSON.parse(updated.themeJson), { palette: { seed: "#556677" } });
  assert.equal(updated.source?.manifest.main, "main.js");
  assert.equal(new TextDecoder().decode(updated.source?.entries.get("main.js")), resources["main.js"]);
  const other = await parseImportedThemeFile(new File([Uint8Array.from(bytes).buffer], "result.toolpkg"), async () => "day");
  assert.deepEqual(JSON.parse(other.themeJson), { palette: { seed: "#abcdef" } });
  assert.equal(updated.metadata?.themeLicenseText, "Edited agreement");
});

/** Uses HJSON manifests and nested package roots through the ordinary declaration paths. */
test("reads and updates templates in a nested HJSON ToolPkg", async () => {
  const data = { toolpkg_id: "org.operit.nested", version: "1", themes: [night] };
  const nestedResources = Object.fromEntries(Object.entries(resources).map(([path, bytes]) => [`bundle/${path}`, bytes]));
  const file = archiveFile(data, nestedResources, "bundle/manifest.hjson",
    `{toolpkg_id:'org.operit.nested',version:'1',themes:[{id:'night',path:'templates/night/visual.json',preview:'templates/night/cover.png'}]}`);
  const source = (await parseImportedThemeFile(file)).source;
  assert.ok(source);
  assert.equal(source.manifestPath, "bundle/manifest.hjson");
  const authored = archiveFile({ toolpkg_id: "org.operit.nested", themes: [{id:"default",path:"theme.json",preview:"preview.png"}] }, {
    "theme.json": JSON.stringify({ palette: { seed: "#778899" } }), "preview.png":"preview",
  });
  const updated = await updateToolPkgTheme(source, new Uint8Array(await authored.arrayBuffer()));
  const parsed = await parseImportedThemeFile(new File([Uint8Array.from(updated).buffer], "updated.toolpkg"));
  assert.deepEqual(JSON.parse(parsed.themeJson), { palette: { seed: "#778899" } });
  assert.equal(parsed.source?.manifestPath, "bundle/manifest.hjson");
  assert.equal(new TextDecoder().decode(parsed.source?.entries.get("bundle/main.js")), resources["main.js"]);
});

/** Allows the same nonblank package IDs as the shared ToolPkg parser. */
test("accepts ordinary ToolPkg IDs including underscores and short names", () => {
  for (const id of ["com.operit.character_cards", "com.operit.context_limiter", "arbitrary_package", "my-tools"]) {
    assert.doesNotThrow(() => validateToolPkgId(id));
  }
  for (const id of ["", " ", null, 12]) assert.throws(() => validateToolPkgId(id), /nonblank text/);
});

/** Preserves untouched locales, author entries and the existing agreement reference. */
test("visual-only edits retain complete package metadata", async () => {
  const metadata = manifest([night, day]);
  metadata.toolpkg_id = "com.operit.character_cards";
  metadata.display_name = { zh: "Chinese name", en: "English name" };
  metadata.description = { values: { zh: "Chinese description", en: "English description" } };
  metadata.author = ["Author A", "Author B"];
  const source = (await parseImportedThemeFile(archiveFile(metadata, resources), async () => "night")).source!;
  const authoredMetadata = {
    toolpkg_id: metadata.toolpkg_id, version: metadata.version,
    display_name: "Chinese name", description: "Chinese description", author: ["Author A, Author B"],
    license: "licenses/agreement.txt",
    themes: [{ id: "default", path: "theme.json", preview: "preview.png" }],
  };
  const authored = archiveFile(authoredMetadata, {
    "theme.json": "{}", "preview.png": "edited", "licenses/agreement.txt": resources["licenses/agreement.txt"],
  });
  const updated = await updateToolPkgTheme(source, new Uint8Array(await authored.arrayBuffer()));
  const output = await parseImportedThemeFile(new File([Uint8Array.from(updated).buffer], "updated.toolpkg"), async () => "night");
  for (const key of ["toolpkg_id", "display_name", "description", "author", "license"]) {
    assert.deepEqual(output.source!.manifest[key], metadata[key], key);
  }
});

/** Changes only the editable translation when the user explicitly renames the package. */
test("editing Chinese metadata retains the other translations", async () => {
  const data = {toolpkg_id:"local_package",display_name:{values:{zh:"Old",en:"English"}},themes:[night]};
  const original = (await parseImportedThemeFile(archiveFile(data, resources))).source!;
  const authored = archiveFile({toolpkg_id:"local_package",display_name:"New",themes:[{id:"default",path:"theme.json",preview:"preview.png"}]}, {
    "theme.json":"{}","preview.png":"edited",
  });
  const output = await parseImportedThemeFile(new File([Uint8Array.from(await updateToolPkgTheme(original, new Uint8Array(await authored.arrayBuffer()))).buffer], "updated.toolpkg"));
  assert.deepEqual(output.source!.manifest.display_name, {values:{zh:"New",en:"English"}});
});

/** Keeps templates sharing definitions, previews and image bytes intact when editing one contribution. */
test("isolates edited template bytes from shared resources and other package capabilities", async () => {
  const sharedFiles = [{path:"assets/images/shared.png",mediaType:"image/png"}];
  const a = {id:"a",path:"templates/shared.json",preview:"templates/shared.png",files:sharedFiles};
  const b = {...a,id:"b"};
  const original = (await parseImportedThemeFile(archiveFile({toolpkg_id:"shared_package",themes:[a,b],resources:[{key:"cover",path:"templates/shared.png"}]}, {
    "templates/shared.json":'{"background":{"asset":"assets/images/shared.png"}}',
    "templates/shared.png":"original preview", "templates/assets/images/shared.png":"original image",
  }), async () => "a")).source!;
  const authored = archiveFile({toolpkg_id:"shared_package",themes:[{id:"default",path:"theme.json",preview:"preview.png",files:[{path:"assets/images/shared.png",mediaType:"image/png"}]}]}, {
    "theme.json":'{"background":{"asset":"assets/images/shared.png"}}',"preview.png":"edited preview","assets/images/shared.png":"edited image",
  });
  const bytes = await updateToolPkgTheme(original, new Uint8Array(await authored.arrayBuffer()));
  const file = new File([Uint8Array.from(bytes).buffer], "updated.toolpkg");
  const other = await parseImportedThemeFile(file, async () => "b");
  assert.deepEqual(other.source!.template, b);
  assert.equal(new TextDecoder().decode(other.assets[0].bytes), "original image");
  assert.equal(new TextDecoder().decode(other.source!.entries.get("templates/shared.png")), "original preview");
  const edited = await parseImportedThemeFile(file, async () => "a");
  assert.equal(new TextDecoder().decode(edited.assets[0].bytes), "edited image");
  assert.notEqual(edited.source!.template.path, a.path);
  assert.equal(new TextDecoder().decode(edited.source!.entries.get(edited.source!.template.preview)), "edited preview");
});

/** Does not turn absent optional metadata into form defaults during an otherwise visual-only edit. */
test("preserves English-only metadata and absent optional package fields", async () => {
  const data = {toolpkg_id:"simple_package",display_name:{en:"English only"},themes:[night]};
  const source = (await parseImportedThemeFile(archiveFile(data, resources))).source!;
  const authored = archiveFile({toolpkg_id:"simple_package",version:"",display_name:"",author:[""],themes:[{id:"default",path:"theme.json",preview:"preview.png"}]}, {
    "theme.json":"{}","preview.png":"edited",
  });
  const output = await parseImportedThemeFile(new File([Uint8Array.from(await updateToolPkgTheme(source, new Uint8Array(await authored.arrayBuffer()))).buffer], "updated.toolpkg"));
  assert.deepEqual(output.source!.manifest.display_name, data.display_name);
  for (const key of ["author","version","description","license"]) {
    assert.equal(Object.hasOwn(output.source!.manifest,key), false,key);
  }
});

/** Keeps full agreement text and author spelling unchanged during metadata projection. */
test("retains agreement whitespace and original author spelling", async () => {
  const data = {toolpkg_id:"plain_package",author:[" Author A ","Author B"],license:"legal/original.txt",themes:[night]};
  const agreement = "  Agreement title\n\nTerms\n";
  const source = (await parseImportedThemeFile(archiveFile(data,{...resources,"legal/original.txt":agreement}))).source!;
  const authored = archiveFile({toolpkg_id:"plain_package",author:[" Author A , Author B"],license:"licenses/agreement.txt",themes:[{id:"default",path:"theme.json",preview:"preview.png"}]},{
    "theme.json":"{}","preview.png":"edited","licenses/agreement.txt":agreement,
  });
  const output = await parseImportedThemeFile(new File([Uint8Array.from(await updateToolPkgTheme(source,new Uint8Array(await authored.arrayBuffer()))).buffer],"updated.toolpkg"));
  assert.equal(output.source!.manifest.license,data.license);
  assert.deepEqual(output.source!.manifest.author,data.author);
  assert.equal(output.metadata!.themeLicenseText,agreement);
});
