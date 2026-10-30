/** Versioned, offline interchange shared by Studio, the capture CLI and Figma. */
import schema from "../../../core/crates/plugin/theme/schema/theme-v1.schema.json" with { type: "json" };

export type Json =
  null | boolean | number | string | Json[] | { [key: string]: Json };
export type Theme = { [key: string]: Json };
export interface DesignAsset {
  path: string;
  mediaType: string;
  base64: string;
}
export interface Binding {
  path: string;
  fallback?: string;
  property:
    | "fill"
    | "textColor"
    | "paddingLeft"
    | "paddingRight"
    | "cornerRadius"
    | "fontSize"
    | "width"
    | "height"
    | "image";
  boolean?: boolean;
}
export interface TextRun {
  start: number;
  end: number;
  color?: string;
  fontSize?: number;
  fontFamily?: string;
  fontWeight?: number;
  italic?: boolean;
}
export interface DesignNode {
  id: string;
  name: string;
  type: "frame" | "rectangle" | "ellipse" | "text" | "image" | "vector";
  x: number;
  y: number;
  width: number;
  height: number;
  rotation?: number;
  fill?: string;
  opacity?: number;
  radius?: number;
  stroke?: string;
  strokeWidth?: number;
  text?: string;
  fontSize?: number;
  fontFamily?: string;
  fontWeight?: number;
  italic?: boolean;
  runs?: TextRun[];
  textAlign?: "LEFT" | "CENTER" | "RIGHT" | "JUSTIFIED";
  lineHeight?: number;
  asset?: string;
  svg?: string;
  imageFit?: "FILL" | "FIT" | "CROP" | "TILE";
  clipsContent?: boolean;
  paddingLeft?: number;
  paddingRight?: number;
  paddingTop?: number;
  paddingBottom?: number;
  source?: { [key: string]: Json };
  bindings?: Binding[];
  children?: DesignNode[];
}
export interface DesignChange {
  nodeId: string;
  name: string;
  source?: DesignNode["source"];
  before: DesignNode | null;
  after: DesignNode | null;
  parentBefore?: string | null;
  parentAfter?: string | null;
  indexBefore?: number;
  indexAfter?: number;
}
export interface DesignDocument {
  format: "operit-design";
  version: 1;
  name: string;
  origin: "theme-studio" | "flutter-runtime" | "figma";
  theme?: Theme;
  assets: DesignAsset[];
  scenes: DesignNode[];
  warnings: string[];
  changes?: DesignChange[];
  softwareContext?: {
    packId: string;
    index: Json;
    entry: { id: string; name: string; group: string; file: string };
  };
  studioContext?: {
    metadata: Record<string, string>;
    toolpkg?: { base64: string; templateId: string };
  };
}
export const MAX_FILE_BYTES = 64 * 1024 * 1024;
const SAFE_PATH = /^assets\/(images|fonts)\/[a-zA-Z0-9_./-]+$/;
const HEX = /^#[0-9A-Fa-f]{8}$/;
const own = (value: object, key: string) =>
  Object.prototype.hasOwnProperty.call(value, key);
export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value));
}
export function getPath(root: Theme, path: string): Json | undefined {
  let current: unknown = root;
  for (const part of path.split(".")) {
    if (!current || typeof current !== "object" || !own(current, part))
      return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return current as Json | undefined;
}
export function setPath(root: Theme, path: string, value: Json): void {
  const parts = path.split(".");
  if (
    parts.some(
      (p) => !p || ["__proto__", "prototype", "constructor"].includes(p),
    )
  )
    throw new Error("非法字段路径");
  let current = root;
  for (const part of parts.slice(0, -1)) {
    const child = current[part];
    if (!child || typeof child !== "object" || Array.isArray(child))
      throw new Error(`字段不存在：${path}`);
    current = child;
  }
  if (!own(current, parts[parts.length - 1]!))
    throw new Error(`字段不存在：${path}`);
  current[parts[parts.length - 1]!] = value;
}
export function leaves(
  root: Theme,
  prefix = "",
): Array<{ path: string; value: Json }> {
  return Object.entries(root).flatMap(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return value && typeof value === "object" && !Array.isArray(value)
      ? leaves(value, path)
      : [{ path, value }];
  });
}
/** Validates exactly the keyword set used by the repository's visual schema. Rust still validates media and export. */
export function validateTheme(theme: unknown): asserts theme is Theme {
  const root = schema as unknown as Record<string, any>;
  function check(
    value: unknown,
    rule: Record<string, any>,
    path: string,
  ): void {
    if (rule.$ref) {
      if (!rule.$ref.startsWith("#/$defs/"))
        throw new Error("未知 schema 引用");
      return check(value, root.$defs[rule.$ref.slice(8)], path);
    }
    const types = Array.isArray(rule.type) ? rule.type : [rule.type];
    const kind =
      value === null ? "null" : Array.isArray(value) ? "array" : typeof value;
    if (
      !types.includes(kind) &&
      !(
        kind === "number" &&
        types.includes("integer") &&
        Number.isInteger(value)
      )
    )
      throw new Error(`${path}: 类型不符合主题契约`);
    if (rule.enum && !rule.enum.includes(value))
      throw new Error(`${path}: 不支持的选项`);
    if (
      typeof value === "number" &&
      (!Number.isFinite(value) ||
        value < (rule.minimum ?? -Infinity) ||
        value > (rule.maximum ?? Infinity))
    )
      throw new Error(`${path}: 数值超出范围`);
    if (
      typeof value === "string" &&
      (value.length < (rule.minLength ?? 0) ||
        value.length > (rule.maxLength ?? Infinity) ||
        (rule.pattern && !new RegExp(rule.pattern).test(value)))
    )
      throw new Error(`${path}: 格式不符合主题契约`);
    if (kind === "object") {
      const obj = value as Record<string, unknown>;
      for (const key of rule.required ?? [])
        if (!own(obj, key)) throw new Error(`${path}.${key}: 缺少字段`);
      for (const [key, child] of Object.entries(obj)) {
        if (!own(rule.properties ?? {}, key)) {
          if (rule.additionalProperties === false)
            throw new Error(`${path}.${key}: 未知字段`);
        } else check(child, rule.properties[key], `${path}.${key}`);
      }
    }
  }
  check(theme, root, "theme");
}
export function parseDocument(text: string): DesignDocument {
  if (text.length > MAX_FILE_BYTES) throw new Error("设计文件超过 64 MiB");
  return validateDocument(JSON.parse(text));
}
export function validateDocument(value: unknown): DesignDocument {
  if (!value || typeof value !== "object")
    throw new Error("设计文件必须是对象");
  const doc = value as DesignDocument;
  if (doc.format !== "operit-design" || doc.version !== 1)
    throw new Error("不是支持的 Operit 设计文件（需要 version 1）");
  if (
    typeof doc.name !== "string" ||
    doc.name.length > 512 ||
    !["theme-studio", "flutter-runtime", "figma"].includes(doc.origin)
  )
    throw new Error("设计文件元数据无效");
  if (
    !Array.isArray(doc.assets) ||
    !Array.isArray(doc.scenes) ||
    !Array.isArray(doc.warnings) ||
    doc.warnings.some((w) => typeof w !== "string")
  )
    throw new Error("设计文件结构无效");
  if (doc.theme) validateTheme(doc.theme);
  const assets = new Set<string>();
  let bytes = 0;
  for (const asset of doc.assets) {
    if (
      typeof asset.path !== "string" ||
      !SAFE_PATH.test(asset.path) ||
      asset.path.split("/").some((p) => p === "." || p === "..") ||
      assets.has(asset.path)
    )
      throw new Error(`素材路径无效或重复：${asset.path}`);
    if (
      !["image/png", "image/jpeg", "image/svg+xml", "font/ttf"].includes(
        asset.mediaType,
      ) ||
      typeof asset.base64 !== "string" ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(
        asset.base64,
      )
    )
      throw new Error(`素材编码无效：${asset.path}`);
    bytes += asset.base64.length;
    if (bytes > MAX_FILE_BYTES) throw new Error("素材总量超过限制");
    assets.add(asset.path);
  }
  if (doc.studioContext) {
    const context = doc.studioContext;
    if (
      !context.metadata ||
      typeof context.metadata !== "object" ||
      Object.values(context.metadata).some((v) => typeof v !== "string")
    )
      throw new Error("Studio 元数据无效");
    if (
      context.toolpkg &&
      (typeof context.toolpkg.base64 !== "string" ||
        context.toolpkg.base64.length > MAX_FILE_BYTES ||
        !/^[A-Za-z0-9+/]*={0,2}$/.test(context.toolpkg.base64) ||
        typeof context.toolpkg.templateId !== "string")
    )
      throw new Error("原始 ToolPkg 上下文无效");
  }
  if (doc.theme)
    for (const { value } of leaves(doc.theme))
      if (
        typeof value === "string" &&
        value.startsWith("assets/") &&
        !assets.has(value)
      )
        throw new Error(`主题缺少素材：${value}`);
  const ids = new Set<string>();
  function visit(node: DesignNode, depth: number): void {
    if (
      !node ||
      depth > 100 ||
      ids.size >= 12000 ||
      typeof node.id !== "string" ||
      ids.has(node.id)
    )
      throw new Error("图层 ID 重复、过多或嵌套过深");
    ids.add(node.id);
    if (
      typeof node.name !== "string" ||
      !["frame", "rectangle", "ellipse", "text", "image", "vector"].includes(
        node.type,
      )
    )
      throw new Error(`图层无效：${node.id}`);
    for (const key of ["x", "y", "width", "height"] as const)
      if (
        !Number.isFinite(node[key]) ||
        Math.abs(node[key]) > 100000 ||
        ((key === "width" || key === "height") && node[key] < 0)
      )
        throw new Error(`图层尺寸无效：${node.id}`);
    for (const key of [
      "opacity",
      "radius",
      "strokeWidth",
      "fontSize",
      "lineHeight",
      "paddingLeft",
      "paddingRight",
    ] as const)
      if (
        node[key] !== undefined &&
        (!Number.isFinite(node[key]) || node[key]! < 0 || node[key]! > 10000)
      )
        throw new Error(`图层属性无效：${node.id}.${key}`);
    if (
      node.rotation !== undefined &&
      (!Number.isFinite(node.rotation) || Math.abs(node.rotation) > 360)
    )
      throw new Error("旋转角度无效");
    if (node.opacity !== undefined && node.opacity > 1)
      throw new Error("透明度超出范围");
    for (const color of [node.fill, node.stroke])
      if (color !== undefined && !HEX.test(color))
        throw new Error("图层颜色无效");
    if (node.asset && !assets.has(node.asset))
      throw new Error(`图层缺少素材：${node.asset}`);
    if (
      node.text !== undefined &&
      (typeof node.text !== "string" || node.text.length > 1000000)
    )
      throw new Error("图层文字无效");
    if (
      node.svg !== undefined &&
      (typeof node.svg !== "string" ||
        node.svg.length > 2000000 ||
        /<script\b|<foreignObject\b|\bon\w+\s*=|(?:href|url)\s*[=(]\s*["']?(?:https?:|javascript:|data:)/i.test(
          node.svg,
        ))
    )
      throw new Error("SVG 包含不支持的内容");
    for (const binding of node.bindings ?? []) {
      if (
        !doc.theme ||
        ![
          "fill",
          "textColor",
          "paddingLeft",
          "paddingRight",
          "cornerRadius",
          "fontSize",
          "width",
          "height",
          "image",
        ].includes(binding.property) ||
        getPath(doc.theme, binding.path) === undefined ||
        (binding.fallback &&
          (!doc.theme || getPath(doc.theme, binding.fallback) === undefined))
      )
        throw new Error(`无效主题绑定：${node.id}`);
    }
    if (node.children && !Array.isArray(node.children))
      throw new Error("子图层无效");
    for (const child of node.children ?? []) visit(child, depth + 1);
  }
  for (const node of doc.scenes) visit(node, 0);
  return doc;
}
export function rgba(hex: string): {
  r: number;
  g: number;
  b: number;
  a: number;
} {
  if (!HEX.test(hex)) throw new Error(`颜色无效：${hex}`);
  return {
    r: parseInt(hex.slice(1, 3), 16) / 255,
    g: parseInt(hex.slice(3, 5), 16) / 255,
    b: parseInt(hex.slice(5, 7), 16) / 255,
    a: parseInt(hex.slice(7, 9), 16) / 255,
  };
}
export function hex(
  color: { r: number; g: number; b: number },
  alpha = 1,
): string {
  return (
    "#" +
    [color.r, color.g, color.b, alpha]
      .map((v) =>
        Math.round(Math.max(0, Math.min(1, v)) * 255)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
      .toUpperCase()
  );
}
export function changedNodes(
  before: DesignNode[],
  after: DesignNode[],
): DesignChange[] {
  const flatten = (roots: DesignNode[]) => {
    const map = new Map<
      string,
      { node: DesignNode; parent: string | null; index: number }
    >();
    const walk = (n: DesignNode, parent: string | null, index: number) => {
      const { children, ...own } = n;
      map.set(n.id, { node: own, parent, index });
      children?.forEach((child, i) => walk(child, n.id, i));
    };
    roots.forEach((n, i) => walk(n, null, i));
    return map;
  };
  const a = flatten(before),
    b = flatten(after);
  return [...new Set([...a.keys(), ...b.keys()])].flatMap((id) => {
    const old = a.get(id),
      next = b.get(id);
    return JSON.stringify(old) === JSON.stringify(next)
      ? []
      : [
          {
            nodeId: id,
            name: (next ?? old)!.node.name,
            source: (next ?? old)!.node.source,
            before: old?.node ?? null,
            after: next?.node ?? null,
            parentBefore: old?.parent ?? null,
            parentAfter: next?.parent ?? null,
            indexBefore: old?.index,
            indexAfter: next?.index,
          },
        ];
  });
}
