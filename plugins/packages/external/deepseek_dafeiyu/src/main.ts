import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";

export const PACKAGE = "com.operit.deepseek_dafeiyu";
export const PET_ROUTE = `toolpkg:${PACKAGE}:ui:pet`;
export const SETTINGS_ROUTE = `toolpkg:${PACKAGE}:ui:settings`;
const WINDOW = "dafeiyu";
export interface Settings {
  width: number;
  x: number;
  y: number;
  animated: boolean;
  walk: boolean;
  clickThrough: boolean;
}
export const DEFAULTS: Settings = { width: 208, x: 100, y: 100, animated: true, walk: false, clickThrough: false };
type Sprites = Record<string, string>;
let configuration: Promise<{ settings: Settings }> | undefined;
let resources: Promise<Sprites> | undefined;

export function validateSettings(value: Settings): Settings {
  for (const key of ["width", "x", "y"] as const) {
    if (!Number.isFinite(value[key])) throw new Error(`${key} 必须是有限数值`);
  }
  if (value.width < 128 || value.width > 320) throw new Error("桌宠宽度必须在 128–320 之间");
  if (Math.abs(value.x) > 100000 || Math.abs(value.y) > 100000) throw new Error("位置超出范围");
  for (const key of ["animated", "walk", "clickThrough"] as const) {
    if (typeof value[key] !== "boolean") throw new Error(`${key} 必须是布尔值`);
  }
  return { width: Math.round(value.width), x: Math.round(value.x), y: Math.round(value.y),
    animated: value.animated, walk: value.walk, clickThrough: value.clickThrough };
}
function config(): Promise<{ settings: Settings }> {
  return configuration ??= PluginConfig.use("dafeiyu", { settings: { ...DEFAULTS } });
}
async function sprites(): Promise<Sprites> {
  if (!resources) {
    resources = Promise.all(["idle", "happy", "eat", "walk", "still"].map(async key =>
      [key, await ToolPkg.readResource(`dafeiyu_${key}`)] as const))
      .then(entries => Object.fromEntries(entries));
    resources.catch(() => { resources = undefined; });
  }
  return resources;
}
function dimensions(settings: Settings): ToolPkg.FloatingWindowOptions {
  return { x: settings.x, y: settings.y, width: settings.width,
    height: Math.ceil(settings.width * 352 / 272) + 44,
    alwaysOnTop: true, clickThrough: settings.clickThrough, focusable: false, draggable: true };
}

/** Explicit user action; permission requests have their own button in settings. */
export async function openPet(settings: Settings): Promise<ToolPkg.FloatingWindowInfo> {
  settings = validateSettings(settings);
  const caps = await ToolPkg.ui.floatingWindows.capabilities();
  if (!caps.supported) throw new Error("当前窗口宿主尚未就绪");
  if (!caps.permissionGranted) throw new Error("请先点击「申请系统悬浮权限」，授权后返回并重试");
  const images = await sprites();
  const current = await ToolPkg.ui.floatingWindows.list();
  if (current.some(window => window.id === WINDOW)) {
    const result = await applyPetSettings(settings);
    await ToolPkg.ui.floatingWindows.show(WINDOW);
    return result;
  }
  return ToolPkg.ui.floatingWindows.open({ id: WINDOW, route: PET_ROUTE,
    state: { settings: { ...settings }, sprites: images }, options: dimensions(settings) });
}

/** Updates the live session and preserves its dragged position and animation state. */
export async function applyPetSettings(settings: Settings): Promise<ToolPkg.FloatingWindowInfo> {
  const checked = validateSettings(settings);
  const { x, y, ...options } = dimensions(checked);
  return ToolPkg.ui.floatingWindows.update(WINDOW, options, { settings: { ...checked } });
}

/** Animated assets play in the host image decoder, without per-frame JS traffic. */
export function pet(ctx: ComposeDslContext): ComposeNode {
  const [settings] = ctx.useState<Settings>("settings", DEFAULTS);
  const [images] = ctx.useState<Sprites>("sprites", {});
  const [mood, setMood] = ctx.useState("mood", "idle");
  const [bubble, setBubble] = ctx.useState("bubble", "拖动我，双击喂小鱼干");
  ctx.useEffect("moodReset", () => {
    const timer = setTimeout(() => { setMood("idle"); setBubble(""); }, mood === "eat" ? 2400 : 1600);
    return () => clearTimeout(timer);
  }, [mood, bubble]);

  const gesture = ctx.Modifier
    .tapGestures({ onTap: () => { setMood("happy"); setBubble("才不是大肥鱼，是大蓝鲸！"); },
      onDoubleTap: () => { setMood("eat"); setBubble("小鱼干！再来一份！"); },
      onLongPress: () => setBubble("在工具箱「大肥鱼桌宠」里调整设置") });
  const animation = !settings.animated ? "still" : mood !== "idle" ? mood : settings.walk ? "walk" : "idle";
  return ctx.UI.Column({ fillMaxSize: true, spacing: 0 }, [
    ctx.UI.Box({ height: 44, fillMaxWidth: true, contentAlignment: "center" }, bubble ? [
      ctx.UI.Box({ padding: 7, backgroundColor: "#F0F5FF" }, [
        ctx.UI.Text({ text: bubble, fontSize: 11, color: "#344878", maxLines: 2 }),
      ]),
    ] : []),
    ctx.UI.Image({ key: "dafeiyu-sprite", uri: images[animation] ?? "", fillMaxWidth: true, weight: 1,
      contentScale: "fit", contentDescription: "大肥鱼：拖动移动，单击跳跃，双击喂食", modifier: gesture }),
  ]);
}

export function settingsPage(ctx: ComposeDslContext): ComposeNode {
  const [settings, setSettings] = ctx.useState<Settings>("settings", { ...DEFAULTS });
  const [preview, setPreview] = ctx.useState("preview", "");
  const [status, setStatus] = ctx.useState("status", "正在读取设置…");
  const [busy, setBusy] = ctx.useState("busy", false);
  const [ready, setReady] = ctx.useState("ready", false);
  const [scope, setScope] = ctx.useState("scope", "");
  const changes = ctx.useRef("changes", { latest: { ...DEFAULTS }, dirty: false, running: false,
    disposed: false, timer: undefined as ReturnType<typeof setTimeout> | undefined });
  ctx.useEffect("releaseSettings", () => () => {
    changes.current.disposed = true;
    clearTimeout(changes.current.timer);
  }, []);
  async function flushChanges(): Promise<void> {
    const pending = changes.current;
    if (pending.running || pending.disposed) return;
    pending.running = true;
    try {
      while (pending.dirty && !pending.disposed) {
        pending.dirty = false;
        const checked = validateSettings(pending.latest), store = await config();
        store.settings = checked;
        await PluginConfig.flush(store);
        const opened = (await ToolPkg.ui.floatingWindows.list()).some(window => window.id === WINDOW);
        if (opened) await applyPetSettings(checked);
        if (!pending.dirty && !pending.disposed) setStatus(opened ? "设置已保存并实时应用" : "设置已保存，打开后生效");
      }
    } catch (error) { if (!pending.disposed) setStatus(String(error)); }
    finally { pending.running = false; }
  }
  function changeSettings(next: Settings, debounce = false): void {
    const checked = validateSettings(next);
    changes.current.latest = checked;
    changes.current.dirty = true;
    setSettings(checked);
    clearTimeout(changes.current.timer);
    if (debounce) changes.current.timer = setTimeout(() => { void flushChanges(); }, 80);
    else void flushChanges();
  }
  const { UI } = ctx;
  async function describe(): Promise<void> {
    const caps = await ToolPkg.ui.floatingWindows.capabilities();
    setScope(caps.scope === "system" ? "系统悬浮 · 可显示在其他应用上方" :
      caps.scope === "application" ? "应用内悬浮 · 离开 Operit 后不显示" : "当前宿主不可用");
    setStatus(caps.permissionGranted ? "已就绪。打开大肥鱼，拖动它试试看。" : "需要系统悬浮窗权限，请授权后返回。");
  }
  async function action(work: () => Promise<unknown>, success: string): Promise<void> {
    if (busy) return;
    setBusy(true);
    try { await work(); if (success) setStatus(success); }
    catch (error) { setStatus(String(error)); }
    finally { setBusy(false); }
  }
  const toggle = (title: string, detail: string, key: "animated" | "walk" | "clickThrough") =>
    UI.Row({ fillMaxWidth: true, spacing: 12, verticalAlignment: "center" }, [
      UI.Column({ weight: 1, spacing: 3 }, [UI.Text({ text: title, style: "titleMedium" }),
        UI.Text({ text: detail, style: "bodySmall", color: ctx.MaterialTheme.colorScheme.onSurfaceVariant })]),
      UI.Switch({ checked: settings[key], enabled: !busy && ready,
        onCheckedChange: value => changeSettings({ ...changes.current.latest, [key]: value }) }),
    ]);
  return UI.LazyColumn({ padding: 20, spacing: 16, fillMaxSize: true,
    onLoad: async () => {
      try {
        const saved = validateSettings((await config()).settings);
        changes.current.latest = saved;
        setSettings(saved);
        setPreview((await sprites()).idle);
        await describe(); setReady(true);
      } catch (error) { setStatus(String(error)); }
    },
  }, [
    UI.Text({ text: "大肥鱼桌宠", style: "headlineMedium" }),
    UI.Text({ text: "让这只吃白饭的小蓝鲸，住在你的桌面上。", style: "bodyLarge" }),
    UI.Card({ fillMaxWidth: true }, [UI.Column({ padding: 16, spacing: 8, horizontalAlignment: "center" }, [
      ...(preview ? [UI.Image({ uri: preview, height: 190, contentScale: "fit", contentDescription: "大肥鱼呼吸动画预览" })] : []),
      UI.Text({ text: scope, style: "labelLarge" }),
      UI.Text({ text: status, style: "bodyMedium" }),
    ])]),
    UI.FlowRow({ spacing: 8 }, [
      UI.Button({ text: "保存并打开 / 应用设置", enabled: ready && !busy, onClick: () => action(async () => {
        const checked = validateSettings(settings), store = await config();
        store.settings = checked; await PluginConfig.flush(store);
        await openPet(checked);
      }, "设置已保存，大肥鱼已打开") }),
      UI.Button({ text: "申请系统悬浮权限", enabled: !busy,
        onClick: () => action(async () => { await ToolPkg.ui.floatingWindows.requestPermission(); await describe(); }, "") }),
      UI.TextButton({ text: "刷新状态", enabled: !busy, onClick: () => action(describe, "") }),
    ]),
    UI.Card({ fillMaxWidth: true }, [UI.Column({ padding: 16, spacing: 16 }, [
      UI.Text({ text: "外观与交互", style: "titleLarge" }),
      UI.Text({ text: `大小 · ${settings.width} 逻辑像素` }),
      UI.Slider({ value: settings.width, min: 128, max: 320, divisions: 12, enabled: ready && !busy,
        onValueChange: value => changeSettings({ ...changes.current.latest, width: Math.round(value) }, true) }),
      toggle("播放动画", "呼吸、摇摆、点击跳跃和喂食；关闭后显示静态形象。", "animated"),
      toggle("行走展示", "播放侧面踏步动画，仍可自由拖动位置。", "walk"),
      toggle("鼠标 / 触摸穿透", "整窗输入透过大肥鱼。可从这个设置页直接关闭穿透。", "clickThrough"),
      UI.Text({ text: "单击跳跃 · 双击喂食 · 按住拖动 · 长按查看提示", style: "bodySmall" }),
    ])]),
    UI.FlowRow({ spacing: 8 }, [
      UI.Button({ text: "隐藏", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.hide(WINDOW), "已隐藏") }),
      UI.Button({ text: "显示", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.show(WINDOW), "已显示") }),
      UI.Button({ text: "关闭", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.close(WINDOW), "已关闭") }),
      UI.TextButton({ text: "恢复默认设置", enabled: !busy, onClick: () => changeSettings({ ...DEFAULTS }) }),
    ]),
    UI.Text({ text: "DeepSeek 社区二创，非官方插件。三视图素材：1190fasheqi/dafeiyu-pet（MIT）。动画由本插件在原素材上制作。", style: "bodySmall" }),
  ]);
}

export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "settings", route: SETTINGS_ROUTE, runtime: "compose_dsl", screen: settingsPage,
    title: { zh: "大肥鱼桌宠", en: "Dafeiyu Pet" } });
  ToolPkg.registerUiRoute({ id: "pet", route: PET_ROUTE, runtime: "compose_dsl", screen: pet,
    title: { zh: "大肥鱼", en: "Dafeiyu" } });
  ToolPkg.registerNavigationEntry({ id: "settings", route: SETTINGS_ROUTE, surface: "toolbox", icon: "SmartToy",
    title: { zh: "大肥鱼桌宠", en: "Dafeiyu Pet" } });
  return true;
}
