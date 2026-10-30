import type { DesignDocument, DesignNode } from "./bridge.ts";
export interface Snapshot {
  protocol: "operit-design-runtime";
  version: 1;
  revision: number;
  document: DesignDocument;
  edits: Array<{
    selector: string;
    properties: Record<string, unknown>;
    kind?: string;
  }>;
}
export type Request =
  | { operation: "snapshot" }
  | { operation: "clear"; target?: string }
  | { operation: "edit"; target: string; properties: Record<string, unknown> }
  | {
      operation: "event";
      event: {
        type: "tap" | "scroll" | "input";
        target: string;
        x?: number;
        y?: number;
        dx?: number;
        dy?: number;
        text?: string;
      };
    };
export type Transport = (request: Request) => Promise<Snapshot>;
const target = (node: DesignNode) =>
  node.source?.runtimeTarget as string | undefined;
const properties = (node: DesignNode) =>
  (node.source?.runtimeProperties ?? []) as string[];
const key = (node: DesignNode) => target(node) ?? node.id;
const px = (n: number | undefined) => `${Number.isFinite(n) ? n : 0}px`;
const color = (s: string | undefined) =>
  s && /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(s) ? s : "transparent";
const cssProperty = (name: string, node: DesignNode) =>
  name === "fill"
    ? node.type === "text"
      ? "color"
      : "background-color"
    : name === "radius"
      ? "border-radius"
      : name.replace(/[A-Z]/g, (s) => "-" + s.toLowerCase());
type Entry = {
  node: DesignNode;
  element: HTMLElement;
  text?: HTMLSpanElement;
  textSignature?: string;
  baseline: Record<string, string>;
};

export function createHttpTransport(
  endpoint: string,
  token: string,
): Transport {
  const url = new URL(endpoint);
  if (
    !["http:", "https:"].includes(url.protocol) ||
    !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)
  )
    throw new Error("连接地址必须指向本机");
  let queue = Promise.resolve();
  return (request) => {
    const next = queue.then(async () => {
      const response = await fetch(new URL("/runtime", url), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(request),
        credentials: "omit",
        referrerPolicy: "no-referrer",
        signal: AbortSignal.timeout(55000),
      });
      const result = await response.json();
      if (!response.ok)
        throw new Error(result.error ?? `连接失败：${response.status}`);
      if (result.protocol !== "operit-design-runtime" || result.version !== 1)
        throw new Error("设计接口版本不兼容");
      return result as Snapshot;
    });
    queue = next.then(
      () => {},
      () => {},
    );
    return next;
  };
}

/** A continuously updated DOM view of the real renderer, with stable inputs. */
export class LiveDesignView {
  private entries = new Map<string, Entry>();
  private observer: MutationObserver;
  private composing = new WeakSet<HTMLElement>();
  private dirty = new WeakSet<HTMLElement>();
  private selected?: string;
  private editing = false;
  private disposed = false;
  private assets = new Map<string, string>();
  private snapshot?: Snapshot;
  onSelection: (node?: DesignNode) => void = () => {};
  onError: (error: unknown) => void = () => {};
  private listeners = new AbortController();
  constructor(
    readonly root: HTMLElement,
    readonly send: Transport,
  ) {
    this.observer = new MutationObserver((records) =>
      this.externalEdits(records),
    );
    this.observe();
    const signal = this.listeners.signal;
    root.addEventListener(
      "click",
      (event) => {
        const entry = this.entryFor(event.target);
        if (!entry) return;
        if (this.editing) {
          event.preventDefault();
          event.stopPropagation();
          this.selected = key(entry.node);
          this.select();
          return;
        }
        if (entry.element instanceof HTMLTextAreaElement) return;
        const rect = entry.element.getBoundingClientRect();
        const clamp = (n: number, max: number) =>
          Math.max(0, Math.min(max - 0.001, n));
        void this.request({
          operation: "event",
          event: {
            type: "tap",
            target: target(entry.node)!,
            x: clamp(
              ((event.clientX - rect.left) * entry.node.width) / rect.width,
              entry.node.width,
            ),
            y: clamp(
              ((event.clientY - rect.top) * entry.node.height) / rect.height,
              entry.node.height,
            ),
          },
        });
      },
      { signal },
    );
    root.addEventListener(
      "wheel",
      (event) => {
        const entry = this.entryFor(event.target);
        if (!entry || this.editing) return;
        event.preventDefault();
        const unit =
          event.deltaMode === 1
            ? 20
            : event.deltaMode === 2
              ? root.clientHeight
              : 1;
        void this.request({
          operation: "event",
          event: {
            type: "scroll",
            target: target(entry.node)!,
            dx: event.deltaX * unit,
            dy: event.deltaY * unit,
          },
        });
      },
      { passive: false, signal },
    );
  }
  get current() {
    return this.snapshot;
  }
  dispose() {
    this.disposed = true;
    this.observer.disconnect();
    this.listeners.abort();
  }
  private observe() {
    if (!this.disposed)
      this.observer.observe(this.root, {
        subtree: true,
        attributes: true,
        attributeFilter: ["style"],
        characterData: true,
        childList: true,
      });
  }
  private entryFor(value: EventTarget | null) {
    const element =
      value instanceof Element
        ? value.closest<HTMLElement>("[data-runtime-key]")
        : null;
    return element ? this.entries.get(element.dataset.runtimeKey!) : undefined;
  }
  private select() {
    for (const [id, entry] of this.entries)
      entry.element.classList.toggle(
        "operit-selected",
        this.editing && id === this.selected,
      );
    this.onSelection(
      this.selected ? this.entries.get(this.selected)?.node : undefined,
    );
  }
  setEditing(value: boolean) {
    this.editing = value;
    this.root.classList.toggle("operit-editing", value);
    this.select();
  }
  async request(request: Request) {
    try {
      const snapshot = await this.send(request);
      if (!this.disposed) this.render(snapshot);
      return snapshot;
    } catch (error) {
      if (!this.disposed) this.onError(error);
      return undefined;
    }
  }
  async edit(values: Record<string, unknown>) {
    const entry = this.selected ? this.entries.get(this.selected) : undefined;
    if (!entry || !target(entry.node)) throw new Error("请先选中控件");
    if (
      Object.keys(values).some((name) => !properties(entry.node).includes(name))
    )
      throw new Error("此控件不支持该属性");
    return this.request({
      operation: "edit",
      target: target(entry.node)!,
      properties: values,
    });
  }
  async clearSelection() {
    const entry = this.selected ? this.entries.get(this.selected) : undefined;
    if (entry)
      return this.request({ operation: "clear", target: target(entry.node)! });
  }
  render(snapshot: Snapshot) {
    if (this.disposed) return;
    if (snapshot.protocol !== "operit-design-runtime" || snapshot.version !== 1)
      throw new Error("界面协议不兼容");
    if (this.snapshot && snapshot.revision < this.snapshot.revision) return;
    this.observer.disconnect();
    try {
      this.snapshot = snapshot;
      this.assets.clear();
      for (const asset of snapshot.document.assets) {
        if (/^image\/(png|jpeg|webp|gif|svg\+xml)$/.test(asset.mediaType))
          this.assets.set(
            asset.path,
            `data:${asset.mediaType};base64,${asset.base64}`,
          );
      }
      const alive = new Set<string>();
      const update = (nodes: DesignNode[], parent: HTMLElement) => {
        let previous: HTMLElement | undefined;
        for (const node of nodes) {
          const id = key(node);
          if (alive.has(id)) throw new Error(`重复界面标识：${id}`);
          alive.add(id);
          const input = node.source?.runtimeInput === true;
          let entry = this.entries.get(id);
          if (entry && entry.element instanceof HTMLTextAreaElement !== input) {
            entry.element.remove();
            this.entries.delete(id);
            entry = undefined;
          }
          if (!entry) {
            entry = {
              node,
              element: document.createElement(input ? "textarea" : "div"),
              baseline: {},
            };
            if (target(node)) entry.element.dataset.runtimeKey = id;
            this.entries.set(id, entry);
            if (entry.element instanceof HTMLTextAreaElement)
              this.input(entry.element, entry);
          }
          entry.node = node;
          const element = entry.element;
          const before = previous ? previous.nextSibling : parent.firstChild;
          if (element !== before) parent.insertBefore(element, before);
          previous = element;
          this.style(entry);
          if (element instanceof HTMLTextAreaElement) {
            element.setAttribute(
              "aria-label",
              String(node.source?.label ?? node.name),
            );
            if (!this.dirty.has(element) && !this.composing.has(element)) {
              const text = node.source?.runtimeObscure ? "" : (node.text ?? "");
              if (element.value !== text) element.value = text;
            }
            element.style.setProperty(
              "-webkit-text-security",
              node.source?.runtimeObscure ? "disc" : "none",
            );
          } else {
            this.text(entry);
            if (node.children?.length) update(node.children, element);
          }
        }
      };
      const scenes = snapshot.document.scenes.filter(
        (node) => node.source?.reference !== true,
      );
      update(scenes, this.root);
      for (const [id, entry] of this.entries)
        if (!alive.has(id)) {
          entry.element.remove();
          this.entries.delete(id);
        }
      this.root.style.width = px(
        Math.max(0, ...scenes.map((node) => node.x + node.width)),
      );
      this.root.style.height = px(
        Math.max(0, ...scenes.map((node) => node.y + node.height)),
      );
      this.select();
    } finally {
      this.observe();
    }
  }
  private text(entry: Entry) {
    const { node, element } = entry;
    if (node.type !== "text") {
      entry.text?.remove();
      entry.text = undefined;
      entry.textSignature = undefined;
      return;
    }
    if (!entry.text) {
      entry.text = document.createElement("span");
      entry.text.className = "operit-text";
      element.prepend(entry.text);
    }
    const signature = JSON.stringify([node.text, node.runs]);
    if (entry.textSignature === signature) return;
    entry.textSignature = signature;
    const text = node.text ?? "";
    if (!node.runs?.length) {
      entry.text.textContent = text;
      return;
    }
    const fragments: Node[] = [];
    let offset = 0;
    for (const run of node.runs) {
      if (run.start > offset)
        fragments.push(document.createTextNode(text.slice(offset, run.start)));
      const span = document.createElement("span");
      span.textContent = text.slice(run.start, run.end);
      if (run.color) span.style.color = color(run.color);
      if (run.fontSize) span.style.fontSize = px(run.fontSize);
      if (run.fontFamily) span.style.fontFamily = run.fontFamily;
      if (run.fontWeight) span.style.fontWeight = String(run.fontWeight);
      if (run.italic) span.style.fontStyle = "italic";
      fragments.push(span);
      offset = run.end;
    }
    if (offset < text.length)
      fragments.push(document.createTextNode(text.slice(offset)));
    entry.text.replaceChildren(...fragments);
  }
  private style(entry: Entry) {
    const { element, node } = entry;
    element.style.cssText = "";
    Object.assign(element.style, {
      position: "absolute",
      boxSizing: "border-box",
      margin: "0",
      padding: "0",
      border: "0",
      left: px(node.x),
      top: px(node.y),
      width: px(node.width),
      height: px(node.height),
      opacity: String(node.opacity ?? 1),
      borderRadius: px(node.radius),
      overflow: node.clipsContent ? "hidden" : "visible",
      pointerEvents:
        target(node) &&
        (node.type !== "frame" || node.fill || !node.children?.length)
          ? "auto"
          : "none",
      backgroundColor: node.type === "text" ? "transparent" : color(node.fill),
      color: node.type === "text" ? color(node.fill) : "inherit",
      fontSize: px(node.fontSize ?? 14),
      fontFamily: node.fontFamily || "sans-serif",
      fontWeight: String(node.fontWeight ?? 400),
      fontStyle: node.italic ? "italic" : "normal",
      lineHeight: node.lineHeight ? px(node.lineHeight) : "normal",
      textAlign: node.textAlign?.toLowerCase() ?? "left",
      whiteSpace: "pre-wrap",
      resize: "none",
    });
    if (node.stroke)
      element.style.border = `${node.strokeWidth ?? 1}px solid ${color(node.stroke)}`;
    if (node.type === "image" && node.asset && this.assets.has(node.asset)) {
      element.style.backgroundImage = `url("${this.assets.get(node.asset)}")`;
      element.style.backgroundSize =
        node.imageFit === "FIT"
          ? "contain"
          : node.imageFit === "CROP"
            ? "cover"
            : "100% 100%";
      element.style.backgroundRepeat = "no-repeat";
    }
    if (node.type === "vector" && node.svg) {
      const bytes = new TextEncoder().encode(node.svg);
      let binary = "";
      for (const byte of bytes) binary += String.fromCharCode(byte);
      element.style.backgroundImage = `url("data:image/svg+xml;base64,${btoa(binary)}")`;
      element.style.backgroundSize = "100% 100%";
    }
    for (const name of properties(node))
      entry.baseline[name] = element.style.getPropertyValue(
        cssProperty(name, node),
      );
  }
  private input(field: HTMLTextAreaElement, entry: Entry) {
    const send = async () => {
      const text = field.value;
      this.dirty.add(field);
      const result = await this.request({
        operation: "event",
        event: { type: "input", target: target(entry.node)!, text },
      });
      if (field.value === text) {
        this.dirty.delete(field);
        if (result && !this.disposed) this.render(result);
      }
    };
    field.addEventListener("compositionstart", () => this.composing.add(field));
    field.addEventListener("compositionend", () => {
      this.composing.delete(field);
      void send();
    });
    field.addEventListener("input", () => {
      if (!this.composing.has(field)) void send();
    });
  }
  /** Captures supported direct DOM edits, without treating our own render as edits. */
  private externalEdits(records: MutationRecord[]) {
    const changes = new Map<Entry, Record<string, unknown>>();
    for (const record of records) {
      const entry = this.entryFor(
        record.target instanceof Text
          ? record.target.parentElement
          : record.target,
      );
      if (!entry) continue;
      const values = changes.get(entry) ?? {},
        allowed = properties(entry.node);
      if (record.type === "attributes") {
        for (const name of allowed) {
          const current = entry.element.style.getPropertyValue(
            cssProperty(name, entry.node),
          );
          if (current === entry.baseline[name] || !current) continue;
          if (name === "fill") {
            const context = document.createElement("canvas").getContext("2d")!;
            context.fillStyle = current;
            context.fillRect(0, 0, 1, 1);
            values.fill =
              "#" +
              [...context.getImageData(0, 0, 1, 1).data]
                .map((n) => n.toString(16).padStart(2, "0"))
                .join("");
          } else {
            const value = parseFloat(current);
            if (Number.isFinite(value)) values[name] = value;
          }
        }
      } else if (
        allowed.includes("text") &&
        entry.text &&
        entry.text.textContent !== entry.node.text
      )
        values.text = entry.text.textContent ?? "";
      if (Object.keys(values).length) changes.set(entry, values);
    }
    for (const [entry, values] of changes)
      void this.request({
        operation: "edit",
        target: target(entry.node)!,
        properties: values,
      });
  }
}
