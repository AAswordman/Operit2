import {
  createHttpTransport,
  LiveDesignView,
  type Snapshot,
} from "./live_client.ts";
import type { DesignNode } from "./bridge.ts";
const find = <T extends HTMLElement>(s: string) =>
  document.querySelector<T>(s)!;
const mode = find<HTMLButtonElement>("#mode"),
  save = find<HTMLButtonElement>("#save"),
  clear = find<HTMLButtonElement>("#clear");
const status = find("#status"),
  connection = find<HTMLFormElement>("#connection"),
  properties = find<HTMLFormElement>("#properties");
const input = connection.querySelector("input")!;
const labels: Record<string, string> = {
  text: "文字",
  fontSize: "字号",
  fill: "颜色（#RRGGBBAA）",
  width: "宽度",
  height: "高度",
  radius: "圆角",
  opacity: "透明度",
  paddingLeft: "左内边距",
  paddingRight: "右内边距",
  paddingTop: "上内边距",
  paddingBottom: "下内边距",
};
let view: LiveDesignView | undefined, current: Snapshot | undefined;
let generation = 0,
  editing = false;
let timer: ReturnType<typeof setTimeout> | undefined;
const message = (text: string, error = false) => {
  status.textContent = text;
  status.classList.toggle("error", error);
};
function select(node?: DesignNode) {
  if (
    properties.contains(document.activeElement) &&
    properties.dataset.target === node?.source?.runtimeTarget
  )
    return;
  properties.replaceChildren();
  properties.dataset.target = String(node?.source?.runtimeTarget ?? "");
  find("#selection").textContent = node?.name ?? "选择一个控件";
  clear.disabled = !node;
  if (!node) return;
  for (const name of (node.source?.runtimeProperties ?? []) as string[]) {
    const label = document.createElement("label");
    label.textContent = labels[name] ?? name;
    const field = document.createElement(
      name === "text" ? "textarea" : "input",
    );
    field.name = name;
    field.value = String(
      node[name as keyof DesignNode] ?? (name === "opacity" ? 1 : ""),
    );
    field.setAttribute("aria-label", labels[name] ?? name);
    label.append(field);
    properties.append(label);
    field.addEventListener("change", () => {
      const value =
        name === "text" || name === "fill" ? field.value : Number(field.value);
      if (typeof value === "number" && !Number.isFinite(value)) {
        message("请输入有效数字", true);
        return;
      }
      void view
        ?.edit({ [name]: value })
        .catch((error) => message(error.message, true));
    });
  }
  find("#saved").textContent =
    node.source?.runtimePersistence === "source"
      ? "修改会按控件来源保留。"
      : "当前控件没有稳定来源标识，重新创建后需重新调整。";
}
async function connect(value: string) {
  const url = new URL(value),
    token = new URLSearchParams(url.hash.slice(1)).get("token");
  if (!token) throw new Error("连接地址缺少口令，请使用本机生成的完整地址");
  const transport = createHttpTransport(url.origin, token),
    id = ++generation;
  clearTimeout(timer);
  view?.dispose();
  find("#viewport").replaceChildren();
  mode.disabled = true;
  save.disabled = true;
  view = new LiveDesignView(find("#viewport"), async (request) => {
    const result = await transport(request);
    if (id !== generation) throw new Error("旧连接已关闭");
    current = result;
    save.disabled = false;
    mode.disabled = false;
    find("#warnings").textContent = result.document.warnings.join("\n");
    message(`已连接 · ${result.document.name}`);
    return result;
  });
  view.onSelection = select;
  view.onError = (error) =>
    message(error instanceof Error ? error.message : String(error), true);
  view.setEditing(editing);
  const active = view;
  message("正在连接…");
  const first = await active.request({ operation: "snapshot" });
  if (!first) return;
  connection.hidden = true;
  input.value = "";
  if (location.hash)
    history.replaceState(null, "", location.pathname + location.search);
  const poll = async () => {
    if (generation !== id) return;
    if (!document.hidden) await active.request({ operation: "snapshot" });
    if (generation === id) timer = setTimeout(poll, 650);
  };
  timer = setTimeout(poll, 650);
}
connection.addEventListener("submit", (event) => {
  event.preventDefault();
  void connect(input.value.trim()).catch((error) =>
    message(error.message, true),
  );
});
mode.addEventListener("click", () => {
  editing = !editing;
  mode.setAttribute("aria-pressed", String(editing));
  mode.textContent = editing ? "继续操作" : "选中编辑";
  find("aside").hidden = !editing;
  view?.setEditing(editing);
});
properties.addEventListener("submit", (event) => event.preventDefault());
clear.addEventListener("click", () => void view?.clearSelection());
save.addEventListener("click", () => {
  if (!current) return;
  const blob = new Blob(
    [
      JSON.stringify(
        { format: "operit-runtime-edits", version: 1, edits: current.edits },
        null,
        2,
      ),
    ],
    { type: "application/json" },
  );
  const url = URL.createObjectURL(blob),
    link = document.createElement("a");
  link.href = url;
  link.download = "operit-runtime-edits.json";
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
});
if (new URLSearchParams(location.hash.slice(1)).has("token"))
  void connect(location.href).catch((error) => message(error.message, true));
