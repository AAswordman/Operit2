import {
  clone,
  parseDocument,
  validateDocument,
  validateTheme,
  leaves,
  getPath,
  setPath,
  rgba,
  hex,
  changedNodes,
  type DesignDocument,
  type DesignNode,
  type Theme,
  type Json,
} from "./bridge.ts";

declare const __html__: string;
figma.showUI(__html__, { width: 420, height: 660, themeColors: true });
const META = "operit.design";
const PART = "operit.source.part";
type Root = FrameNode;
let fonts: Font[] = [];
const packLayouts = new Map<
  string,
  {
    x: number;
    y: number;
    nextX: number;
    nextY: number;
    rowHeight: number;
    count: number;
  }
>();
const loaded = new Set<string>();

function paint(value: string): SolidPaint {
  const { a, ...color } = rgba(value);
  return { type: "SOLID", color, opacity: a };
}
async function font(
  family?: string,
  weight = 400,
  italic = false,
): Promise<FontName> {
  if (!fonts.length) fonts = await figma.listAvailableFontsAsync();
  const desired = weight >= 700 ? "Bold" : weight >= 500 ? "Medium" : "Regular";
  const candidates = fonts.filter(
    (f) => f.fontName.family.toLowerCase() === (family ?? "").toLowerCase(),
  );
  const style = italic ? `${desired} Italic` : desired;
  const result =
    candidates.find((f) => f.fontName.style === style)?.fontName ??
    candidates.find((f) => f.fontName.style === desired)?.fontName ??
    candidates[0]?.fontName ??
    fonts.find(
      (f) => f.fontName.family === "Inter" && f.fontName.style === "Regular",
    )?.fontName ??
    fonts[0]?.fontName;
  if (!result) throw new Error("Figma 中没有可用字体");
  const key = `${result.family}/${result.style}`;
  if (!loaded.has(key)) {
    await figma.loadFontAsync(result);
    loaded.add(key);
  }
  return result;
}
function save(root: Root, document: DesignDocument): void {
  for (const child of [...root.children])
    if (child.getPluginData(PART)) child.remove();
  const text = JSON.stringify(document);
  // One bounded chunk per metadata node avoids pluginData's per-value limit.
  for (let i = 0; i < text.length; i += 16000) {
    const part = figma.createFrame();
    part.name = "Operit source · " + i;
    part.visible = false;
    part.setPluginData(PART, text.slice(i, i + 16000));
    root.appendChild(part);
  }
  root.setPluginData(META, "1");
}
function restore(root: Root): DesignDocument {
  const source = root.children
    .filter((n) => n.getPluginData(PART))
    .map((n) => n.getPluginData(PART))
    .join("");
  if (!source) throw new Error("设计来源数据已被删除，无法安全回传");
  return parseDocument(source);
}
function selectedRoot(): Root {
  let node: BaseNode | null = figma.currentPage.selection[0] ?? null;
  while (node && !node.getPluginData(META)) node = node.parent;
  if (!node || node.type !== "FRAME")
    throw new Error("请先选中 Operit 导入的画板或其中一个图层");
  return node;
}
function originals(doc: DesignDocument): Map<string, DesignNode> {
  const map = new Map<string, DesignNode>();
  const walk = (n: DesignNode) => {
    map.set(n.id, n);
    n.children?.forEach(walk);
  };
  doc.scenes.forEach(walk);
  return map;
}
async function makeVariables(
  root: Root,
  theme: Theme,
): Promise<Map<string, Variable>> {
  const result = new Map<string, Variable>();
  const groups = new Map<string, VariableCollection>();
  for (const { path, value } of leaves(theme)) {
    if (value === null) continue;
    const group = path.startsWith("palette.light.")
      ? "Light"
      : path.startsWith("palette.dark.")
        ? "Dark"
        : "Parameters";
    let collection = groups.get(group);
    if (!collection) {
      collection = figma.variables.createVariableCollection(
        `Operit / ${root.id} / ${group}`,
      );
      groups.set(group, collection);
    }
    const isColor = typeof value === "string" && /^#[0-9A-F]{8}$/.test(value);
    if (typeof value === "object") continue;
    const variable = figma.variables.createVariable(
      path.split(".").join("/"),
      collection,
      isColor
        ? "COLOR"
        : typeof value === "number"
          ? "FLOAT"
          : typeof value === "boolean"
            ? "BOOLEAN"
            : "STRING",
    );
    variable.setValueForMode(
      collection.defaultModeId,
      isColor ? rgba(value as string) : value,
    );
    result.set(path, variable);
  }
  root.setPluginData(
    "operit.variables",
    JSON.stringify([...result].map(([path, v]) => [path, v.id])),
  );
  return result;
}
async function createLayer(
  n: DesignNode,
  doc: DesignDocument,
  variables: Map<string, Variable>,
): Promise<SceneNode> {
  let node: SceneNode;
  if (n.type === "text") node = figma.createText();
  else if (n.type === "ellipse") node = figma.createEllipse();
  else if (n.type === "vector" && n.svg) node = figma.createNodeFromSvg(n.svg);
  else if (n.type === "frame")
    node = n.source?.component ? figma.createComponent() : figma.createFrame();
  else node = figma.createRectangle();
  node.name = n.name;
  node.x = n.x;
  node.y = n.y;
  node.resize(Math.max(0.01, n.width), Math.max(0.01, n.height));
  if (n.rotation !== undefined && "rotation" in node)
    node.rotation = n.rotation;
  node.setPluginData("operit.id", n.id);
  node.visible = true;
  if ("fills" in node && n.type !== "vector")
    node.fills = n.fill ? [paint(n.fill)] : [];
  if (n.opacity !== undefined && "opacity" in node) node.opacity = n.opacity;
  if (n.radius !== undefined && "cornerRadius" in node)
    node.cornerRadius = n.radius;
  if (n.stroke && "strokes" in node) {
    node.strokes = [paint(n.stroke)];
    node.strokeWeight = n.strokeWidth ?? 1;
  }
  if ("clipsContent" in node) node.clipsContent = n.clipsContent ?? false;
  if ("paddingLeft" in node) {
    node.paddingLeft = n.paddingLeft ?? 0;
    node.paddingRight = n.paddingRight ?? 0;
  }
  if (n.type === "text" && node.type === "TEXT") {
    node.fontName = await font(n.fontFamily, n.fontWeight, n.italic);
    if (n.fontFamily && node.fontName.family !== n.fontFamily) {
      const warning = `字体 ${n.fontFamily} 在 Figma 中不可用，已使用 ${node.fontName.family}；原字体素材仍保留，精确排版请安装相应字体。`;
      if (!doc.warnings.includes(warning)) doc.warnings.push(warning);
      n.source = { ...n.source, requestedFont: n.fontFamily };
    }
    // Fix the box before assigning content: a new Figma text node can auto-size
    // when its font or characters change, which otherwise loses captured bounds.
    node.textAutoResize = "NONE";
    node.fontSize = n.fontSize ?? 14;
    node.characters = n.text ?? "";
    node.resize(Math.max(0.01, n.width), Math.max(0.01, n.height));
    node.textAlignHorizontal = n.textAlign ?? "LEFT";
    if (n.lineHeight) node.lineHeight = { unit: "PIXELS", value: n.lineHeight };
    for (const run of n.runs ?? []) {
      if (run.end <= run.start || run.end > node.characters.length) continue;
      if (run.fontFamily || run.fontWeight || run.italic)
        node.setRangeFontName(
          run.start,
          run.end,
          await font(
            run.fontFamily ?? n.fontFamily,
            run.fontWeight ?? n.fontWeight,
            run.italic,
          ),
        );
      if (run.fontSize) node.setRangeFontSize(run.start, run.end, run.fontSize);
      if (run.color) node.setRangeFills(run.start, run.end, [paint(run.color)]);
    }
  }
  if (n.asset && "fills" in node) {
    const asset = doc.assets.find((a) => a.path === n.asset)!;
    if (asset.mediaType === "image/svg+xml") {
      const svg = figma.createNodeFromSvg(
        decodeUtf8(figma.base64Decode(asset.base64)),
      );
      svg.resize(node.width, node.height);
      svg.x = 0;
      svg.y = 0;
      if ("appendChild" in node) {
        node.appendChild(svg);
      } else {
        const replacement = figma.createFrame();
        replacement.name = node.name;
        replacement.resize(node.width, node.height);
        replacement.x = node.x;
        replacement.y = node.y;
        replacement.fills = [];
        replacement.setPluginData("operit.id", n.id);
        replacement.appendChild(svg);
        node.remove();
        node = replacement;
      }
    } else if (asset.mediaType !== "font/ttf") {
      const image = figma.createImage(figma.base64Decode(asset.base64));
      node.fills = [
        {
          type: "IMAGE",
          imageHash: image.hash,
          scaleMode: n.imageFit ?? "FILL",
        },
      ];
      node.setPluginData("operit.imageHash", image.hash);
    }
  }
  for (const binding of n.bindings ?? []) {
    const variable =
      variables.get(binding.path) ??
      (binding.fallback ? variables.get(binding.fallback) : undefined);
    if (
      variable &&
      (binding.property === "fill" || binding.property === "textColor") &&
      variable.resolvedType === "COLOR" &&
      "fills" in node
    ) {
      const fills = node.fills;
      if (fills !== figma.mixed && fills[0]?.type === "SOLID")
        node.fills = [
          figma.variables.setBoundVariableForPaint(fills[0], "color", variable),
          ...fills.slice(1),
        ];
    }
    if (
      variable?.resolvedType === "FLOAT" &&
      ["width", "height", "paddingLeft", "paddingRight", "fontSize"].includes(
        binding.property,
      )
    ) {
      node.setBoundVariable(
        binding.property as VariableBindableNodeField,
        variable,
      );
    }
  }
  if ("appendChild" in node)
    for (const child of n.children ?? [])
      node.appendChild(await createLayer(child, doc, variables));
  if (n.source?.reference === true) node.locked = true;
  return node;
}
async function importDesign(
  text: string,
  software?: DesignDocument["softwareContext"],
): Promise<void> {
  const doc = parseDocument(text);
  if (software) doc.softwareContext = software;
  const root = figma.createFrame();
  root.name = `Operit · ${doc.name}`;
  root.fills = [];
  root.clipsContent = false;
  root.x = figma.viewport.center.x;
  root.y = figma.viewport.center.y;
  const collections: VariableCollection[] = [];
  try {
    const variables = doc.theme
      ? await makeVariables(root, doc.theme)
      : new Map<string, Variable>();
    for (const v of variables.values()) {
      const c = await figma.variables.getVariableCollectionByIdAsync(
        v.variableCollectionId,
      );
      if (c && !collections.includes(c)) collections.push(c);
    }
    for (const scene of doc.scenes)
      root.appendChild(await createLayer(scene, doc, variables));
    root.resize(
      Math.max(1, ...doc.scenes.map((n) => n.x + n.width)),
      Math.max(1, ...doc.scenes.map((n) => n.y + n.height)),
    );
    if (software) {
      root.setPluginData("operit.pack", software.packId);
      let layout = packLayouts.get(software.packId);
      if (!layout) {
        layout = {
          x: root.x,
          y: root.y,
          nextX: root.x,
          nextY: root.y,
          rowHeight: 0,
          count: 0,
        };
        packLayouts.set(software.packId, layout);
      }
      root.x = layout.nextX;
      root.y = layout.nextY;
      layout.nextX += root.width + 160;
      layout.rowHeight = Math.max(layout.rowHeight, root.height);
      layout.count++;
      if (layout.count % 2 === 0) {
        layout.nextX = layout.x;
        layout.nextY += layout.rowHeight + 160;
        layout.rowHeight = 0;
      }
    }
    const baseline = clone(doc);
    baseline.scenes = [];
    for (const child of root.children) {
      const n = await readLayer(child, originals(doc), baseline);
      if (n) baseline.scenes.push(n);
    }
    save(root, baseline);
    figma.currentPage.selection = [root];
    figma.viewport.scrollAndZoomIntoView([root]);
    figma.ui.postMessage({
      type: "loaded",
      name: doc.name,
      theme: doc.theme,
      warnings: doc.warnings,
    });
  } catch (error) {
    root.remove();
    collections.forEach((c) => c.remove());
    throw error;
  }
}
async function readVariables(root: Root, theme: Theme): Promise<void> {
  const entries = JSON.parse(
    root.getPluginData("operit.variables") || "[]",
  ) as [string, string][];
  for (const [path, id] of entries) {
    const v = await figma.variables.getVariableByIdAsync(id);
    if (!v) throw new Error(`主题变量被删除：${path}`);
    const c = await figma.variables.getVariableCollectionByIdAsync(
      v.variableCollectionId,
    );
    if (!c) throw new Error(`主题变量集合被删除：${path}`);
    const value = v.valuesByMode[c.defaultModeId];
    if (typeof value === "object" && !("r" in value))
      throw new Error(`暂不支持跨集合变量别名：${path}`);
    setPath(
      theme,
      path,
      typeof value === "object"
        ? hex(value, "a" in value ? value.a : 1)
        : (value as Json),
    );
  }
}
async function readLayer(
  node: SceneNode,
  base: Map<string, DesignNode>,
  doc: DesignDocument,
  seen = new Set<string>(),
): Promise<DesignNode | null> {
  if (node.getPluginData(PART) || !node.visible) return null;
  const sourceId = node.getPluginData("operit.id") || `figma-${node.id}`;
  const copied = seen.has(sourceId),
    id = copied ? `figma-${node.id}` : sourceId;
  seen.add(id);
  const copySource =
    node.getPluginData("operit.copySource") || (copied ? sourceId : "");
  if (copied) {
    node.setPluginData("operit.id", id);
    node.setPluginData("operit.copySource", sourceId);
  }
  const original = base.get(sourceId) ?? base.get(copySource);
  const n: DesignNode = original
    ? clone(original)
    : {
        id,
        name: node.name,
        type:
          node.type === "TEXT"
            ? "text"
            : node.type === "ELLIPSE"
              ? "ellipse"
              : "frame",
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        source: { kind: "figma-new", figmaNode: node.id },
      };
  delete n.children;
  n.id = id;
  if (copySource) {
    n.source = {
      ...n.source,
      kind: "figma-copy",
      copiedFrom: copySource,
      figmaNode: node.id,
    };
    delete n.bindings;
  }
  n.name = node.name;
  n.x = node.x;
  n.y = node.y;
  n.width = node.width;
  n.height = node.height;
  if ("opacity" in node && (node.opacity !== 1 || n.opacity !== undefined))
    n.opacity = node.opacity;
  if (
    "cornerRadius" in node &&
    node.cornerRadius !== figma.mixed &&
    (node.cornerRadius !== 0 || n.radius !== undefined)
  )
    n.radius = node.cornerRadius;
  if (
    "clipsContent" in node &&
    (node.clipsContent || n.clipsContent !== undefined)
  )
    n.clipsContent = node.clipsContent;
  if ("paddingLeft" in node && n.paddingLeft !== undefined)
    n.paddingLeft = node.paddingLeft;
  if ("paddingRight" in node && n.paddingRight !== undefined)
    n.paddingRight = node.paddingRight;
  if ("fills" in node && node.fills !== figma.mixed) {
    const fills = node.fills.filter((f) => f.visible !== false);
    const fill = fills[0];
    if (fill?.type === "SOLID") n.fill = hex(fill.color, fill.opacity ?? 1);
    else if (!fill) delete n.fill;
    if (fill?.type === "IMAGE" && fill.imageHash) {
      n.type = "image";
      if (
        fill.imageHash !== node.getPluginData("operit.imageHash") ||
        !n.asset
      ) {
        let bytes = await figma.getImageByHash(fill.imageHash)!.getBytesAsync();
        const jpeg = bytes[0] === 0xff && bytes[1] === 0xd8;
        if (
          !jpeg &&
          !(
            bytes[0] === 0x89 &&
            bytes[1] === 0x50 &&
            bytes[2] === 0x4e &&
            bytes[3] === 0x47
          )
        )
          bytes = await node.exportAsync({ format: "PNG" });
        const path = `assets/images/figma_${node.id.split(":").join("_")}.${jpeg ? "jpg" : "png"}`;
        doc.assets = doc.assets.filter((a) => a.path !== path);
        doc.assets.push({
          path,
          mediaType: jpeg ? "image/jpeg" : "image/png",
          base64: figma.base64Encode(bytes),
        });
        n.asset = path;
      }
      n.imageFit = fill.scaleMode;
    }
    if (fills.length > 1 || (fill && !["SOLID", "IMAGE"].includes(fill.type))) {
      n.source = {
        ...n.source,
        figmaAppearance: {
          fills: clone(fills) as unknown as Json,
          figmaNode: node.id,
        },
      };
      doc.warnings.push(
        `${node.name} 包含复杂填充；source.figmaAppearance 保留实际参数，主题仅接收已有映射。`,
      );
    }
  }
  if ("strokes" in node) {
    const s = node.strokes[0];
    if (s?.type === "SOLID") {
      n.stroke = hex(s.color, s.opacity ?? 1);
      if (typeof node.strokeWeight === "number")
        n.strokeWidth = node.strokeWeight;
    }
  }
  if (node.type === "TEXT") {
    n.text = node.characters;
    if (node.fontSize !== figma.mixed) n.fontSize = node.fontSize;
    if (node.fontName !== figma.mixed) {
      n.fontFamily = node.fontName.family;
      n.fontWeight = /bold/i.test(node.fontName.style)
        ? 700
        : /medium/i.test(node.fontName.style)
          ? 500
          : 400;
      n.italic = /italic/i.test(node.fontName.style);
    }
    n.textAlign = node.textAlignHorizontal;
    if (node.lineHeight !== figma.mixed && node.lineHeight.unit === "PIXELS")
      n.lineHeight = node.lineHeight.value;
    n.runs = node
      .getStyledTextSegments(["fontName", "fontSize", "fills"])
      .map((r) => ({
        start: r.start,
        end: r.end,
        fontSize: r.fontSize,
        fontFamily: r.fontName.family,
        fontWeight: /bold/i.test(r.fontName.style)
          ? 700
          : /medium/i.test(r.fontName.style)
            ? 500
            : 400,
        italic: /italic/i.test(r.fontName.style),
        ...(r.fills[0]?.type === "SOLID"
          ? { color: hex(r.fills[0].color, r.fills[0].opacity ?? 1) }
          : {}),
      }));
  }
  if (
    original?.type === "vector" ||
    (!original &&
      ["VECTOR", "BOOLEAN_OPERATION", "STAR", "POLYGON", "LINE"].includes(
        node.type,
      ))
  ) {
    n.type = "vector";
    n.svg = await node.exportAsync({ format: "SVG_STRING" });
    delete n.asset;
    return n;
  }
  if ("children" in node) {
    // SVG image wrappers preserve their source asset unless edited; export the
    // actual vector subtree so edits are retained in the handoff.
    if (original?.type === "image" && original.asset?.endsWith(".svg")) {
      const svg = await node.exportAsync({ format: "SVG_STRING" });
      const path = `assets/images/figma_${node.id.split(":").join("_")}.svg`;
      doc.assets = doc.assets.filter((a) => a.path !== path);
      doc.assets.push({
        path,
        mediaType: "image/svg+xml",
        base64: figma.base64Encode(utf8(svg)),
      });
      n.asset = path;
      n.type = "image";
    } else {
      const children: DesignNode[] = [];
      for (const child of node.children) {
        const n = await readLayer(child, base, doc, seen);
        if (n) children.push(n);
      }
      n.children = children;
    }
  }
  if ("rotation" in node && (node.rotation !== 0 || n.rotation !== undefined))
    n.rotation = node.rotation;
  if ("effects" in node && node.effects.length) {
    const prior = n.source?.figmaAppearance as Record<string, Json> | undefined;
    n.source = {
      ...n.source,
      figmaAppearance: {
        ...prior,
        effects: clone(node.effects) as unknown as Json,
        figmaNode: node.id,
      },
    };
  }
  if ("layoutMode" in node && node.layoutMode !== "NONE") {
    const prior = n.source?.figmaAppearance as Record<string, Json> | undefined;
    n.source = {
      ...n.source,
      figmaAppearance: {
        ...prior,
        layoutMode: node.layoutMode,
        itemSpacing: node.itemSpacing,
        paddingTop: node.paddingTop,
        paddingBottom: node.paddingBottom,
        paddingLeft: node.paddingLeft,
        paddingRight: node.paddingRight,
        figmaNode: node.id,
      },
    };
  }
  return n;
}
function decodeUtf8(bytes: Uint8Array): string {
  return decodeURIComponent(
    Array.from(bytes, (b) => "%" + b.toString(16).padStart(2, "0")).join(""),
  );
}
function utf8(text: string): Uint8Array {
  const encoded = encodeURIComponent(text);
  const bytes: number[] = [];
  for (let i = 0; i < encoded.length; i++) {
    if (encoded[i] === "%") {
      bytes.push(parseInt(encoded.slice(i + 1, i + 3), 16));
      i += 2;
    } else bytes.push(encoded.charCodeAt(i));
  }
  return new Uint8Array(bytes);
}
function collectThemeChanges(
  doc: DesignDocument,
  base: Map<string, DesignNode>,
  bound: Set<string>,
): void {
  if (!doc.theme) return;
  const edits = new Map<string, Json>();
  const walk = (n: DesignNode) => {
    const old = base.get(n.id);
    if (old)
      for (const b of old.bindings ?? []) {
        if (bound.has(`${n.id}:${b.property}`)) continue;
        const prop =
          b.property === "cornerRadius"
            ? "radius"
            : b.property === "textColor"
              ? "fill"
              : b.property === "image"
                ? "asset"
                : b.property;
        const after = n[prop as keyof DesignNode],
          before = old[prop as keyof DesignNode];
        if (after === before || after === undefined) continue;
        const value = b.boolean ? Number(after) > 0 : (after as Json);
        // Bound variable edits are already projected through readVariables.
        const current = getPath(doc.theme!, b.path);
        if (
          current === null &&
          b.fallback &&
          value === getPath(doc.theme!, b.fallback)
        )
          continue;
        if (current === value) continue;
        if (
          edits.has(b.path) &&
          JSON.stringify(edits.get(b.path)) !== JSON.stringify(value)
        )
          throw new Error(
            `${b.path} 在多个画板中被改成不同值。请统一该公共参数后再导出。`,
          );
        edits.set(b.path, value);
      }
    n.children?.forEach(walk);
  };
  doc.scenes.forEach(walk);
  for (const [path, value] of edits) setPath(doc.theme, path, value);
  validateTheme(doc.theme);
}
async function readDesign(root: Root): Promise<DesignDocument> {
  const source = restore(root),
    doc = clone(source),
    base = originals(source);
  doc.origin = "figma";
  doc.warnings = [...source.warnings];
  if (doc.theme) await readVariables(root, doc.theme);
  doc.scenes = [];
  const seen = new Set<string>();
  for (const node of root.children) {
    const scene = await readLayer(node, base, doc, seen);
    if (scene) doc.scenes.push(scene);
  }
  const bound = new Set<string>();
  const variableIds = new Map<string, string>(
    JSON.parse(root.getPluginData("operit.variables") || "[]"),
  );
  function readBindings(node: SceneNode): void {
    const id = node.getPluginData("operit.id");
    for (const b of base.get(id)?.bindings ?? []) {
      const variable =
        variableIds.get(b.path) ??
        (b.fallback ? variableIds.get(b.fallback) : undefined);
      if (!variable) continue;
      if (
        (b.property === "fill" || b.property === "textColor") &&
        "fills" in node &&
        node.fills !== figma.mixed &&
        node.fills[0]?.type === "SOLID" &&
        node.fills[0].boundVariables?.color?.id === variable
      )
        bound.add(`${id}:${b.property}`);
      else if (
        node.boundVariables?.[b.property as VariableBindableNodeField]?.id ===
        variable
      )
        bound.add(`${id}:${b.property}`);
    }
    if ("children" in node) node.children.forEach(readBindings);
  }
  root.children.forEach(readBindings);
  collectThemeChanges(doc, base, bound);
  doc.changes = changedNodes(source.scenes, doc.scenes);
  doc.warnings = [...new Set(doc.warnings)];
  validateDocument(doc);
  return doc;
}
async function exportDesign(): Promise<void> {
  const doc = await readDesign(selectedRoot());
  figma.ui.postMessage({
    type: "download",
    filename: `${doc.name.replace(/[^\p{L}\p{N}_-]/gu, "_")}.operit-design.json`,
    text: JSON.stringify(doc, null, 2),
    changes: doc.changes?.length ?? 0,
    warnings: doc.warnings,
  });
}
async function exportSoftware(): Promise<void> {
  const selected = selectedRoot();
  const packId = selected.getPluginData("operit.pack");
  if (!packId) throw new Error("当前画板不属于软件设计包，请使用单画板导出");
  const roots = figma.currentPage.findAll(
    (n) => n.type === "FRAME" && n.getPluginData("operit.pack") === packId,
  ) as Root[];
  const documents = [];
  const entryIds = new Set<string>();
  let index: Json | undefined;
  for (const root of roots) {
    const document = await readDesign(root);
    if (!document.softwareContext) throw new Error("软件包来源缺失");
    index ??= document.softwareContext.index;
    const originalId = document.softwareContext.entry.id;
    const entryId = entryIds.has(originalId)
      ? `${originalId}:${root.id}`
      : originalId;
    entryIds.add(entryId);
    documents.push({
      entry: {
        ...document.softwareContext.entry,
        id: entryId,
      },
      document,
    });
  }
  figma.ui.postMessage({ type: "download-pack", index, documents });
}
async function applyTheme(text: string): Promise<void> {
  const root = selectedRoot(),
    source = restore(root),
    theme = JSON.parse(text);
  validateTheme(theme);
  if (!source.theme)
    throw new Error("运行时采集没有绑定主题，请在主题组件稿中编辑主题参数");
  // Preserve all existing graph edits; only update fields explicitly changed in
  // the parameter editor. Enum and asset changes are retained for real preview.
  const oldTheme = source.theme;
  const variableIds = JSON.parse(
    root.getPluginData("operit.variables") || "[]",
  ) as [string, string][];
  const retained: [string, string][] = [];
  for (const [path, id] of variableIds) {
    const value = getPath(theme, path);
    if (value === undefined || value === null) continue;
    retained.push([path, id]);
    const v = await figma.variables.getVariableByIdAsync(id);
    if (!v) continue;
    const c = await figma.variables.getVariableCollectionByIdAsync(
      v.variableCollectionId,
    );
    if (!c) continue;
    v.setValueForMode(
      c.defaultModeId,
      v.resolvedType === "COLOR"
        ? rgba(value as string)
        : (value as string | number | boolean),
    );
  }
  const collection = retained.length
    ? null
    : figma.variables.createVariableCollection(
        `Operit / ${root.id} / Parameters`,
      );
  let extraCollection = collection;
  for (const { path, value } of leaves(theme)) {
    if (
      value === null ||
      typeof value === "object" ||
      retained.some(([p]) => p === path)
    )
      continue;
    extraCollection ??= figma.variables.createVariableCollection(
      `Operit / ${root.id} / Added parameters`,
    );
    const isColor = typeof value === "string" && /^#[0-9A-F]{8}$/.test(value);
    const v = figma.variables.createVariable(
      path.split(".").join("/"),
      extraCollection,
      isColor
        ? "COLOR"
        : typeof value === "number"
          ? "FLOAT"
          : typeof value === "boolean"
            ? "BOOLEAN"
            : "STRING",
    );
    v.setValueForMode(
      extraCollection.defaultModeId,
      isColor ? rgba(value as string) : value,
    );
    retained.push([path, v.id]);
  }
  root.setPluginData("operit.variables", JSON.stringify(retained));
  const variables = new Map<string, Variable>();
  for (const [path, id] of retained) {
    const v = await figma.variables.getVariableByIdAsync(id);
    if (v) variables.set(path, v);
  }
  const base = originals(source);
  function update(node: SceneNode): void {
    const model = base.get(node.getPluginData("operit.id"));
    for (const b of model?.bindings ?? []) {
      const value = getPath(theme, b.path);
      if (JSON.stringify(value) === JSON.stringify(getPath(oldTheme, b.path)))
        continue;
      if (
        (b.property === "fill" || b.property === "textColor") &&
        "fills" in node
      ) {
        const effective =
          value ?? (b.fallback ? getPath(theme, b.fallback) : undefined);
        if (typeof effective === "string") {
          const variable =
            variables.get(b.path) ??
            (b.fallback ? variables.get(b.fallback) : undefined);
          node.fills = [
            variable?.resolvedType === "COLOR"
              ? figma.variables.setBoundVariableForPaint(
                  paint(effective),
                  "color",
                  variable,
                )
              : paint(effective),
          ];
        }
      }
      if (
        b.property === "cornerRadius" &&
        (node.type === "FRAME" ||
          node.type === "COMPONENT" ||
          node.type === "RECTANGLE" ||
          node.type === "COMPONENT_SET")
      )
        node.cornerRadius = value ? 16 : 0;
      if (
        b.property === "paddingLeft" &&
        "paddingLeft" in node &&
        typeof value === "number"
      )
        node.paddingLeft = value;
      if (
        b.property === "paddingRight" &&
        "paddingRight" in node &&
        typeof value === "number"
      )
        node.paddingRight = value;
    }
    if ("children" in node) node.children.forEach(update);
  }
  root.children.forEach(update);
  source.theme = theme;
  save(root, source);
  figma.ui.postMessage({
    type: "loaded",
    name: source.name,
    theme,
    warnings: [
      "主题参数已保存。样式枚举、字体文件和九宫格切片请回 Theme Studio 用 Flutter 检查真实效果。",
    ],
  });
}
let busy = false;
figma.ui.onmessage = async (message: unknown) => {
  if (!message || typeof message !== "object") return;
  const m = message as {
    type: string;
    text?: string;
    software?: DesignDocument["softwareContext"];
  };
  if (busy) {
    figma.ui.postMessage({ type: "error", message: "上一个操作仍在执行" });
    return;
  }
  busy = true;
  try {
    if (m.type === "import" && typeof m.text === "string")
      await importDesign(m.text, m.software);
    else if (m.type === "export") await exportDesign();
    else if (m.type === "export-software") await exportSoftware();
    else if (m.type === "inspect") {
      const doc = restore(selectedRoot());
      figma.ui.postMessage({
        type: "loaded",
        name: doc.name,
        theme: doc.theme,
        warnings: doc.warnings,
      });
    } else if (m.type === "apply-theme" && typeof m.text === "string")
      await applyTheme(m.text);
  } catch (error) {
    figma.ui.postMessage({ type: "error", message: String(error) });
  } finally {
    busy = false;
    figma.ui.postMessage({ type: "idle" });
  }
};
