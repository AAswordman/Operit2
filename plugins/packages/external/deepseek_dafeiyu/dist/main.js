"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ACTION_DURATION_MS = exports.DEFAULTS = exports.SETTINGS_ROUTE = exports.PET_ROUTE = exports.PACKAGE = void 0;
exports.validateSettings = validateSettings;
exports.openPet = openPet;
exports.applyPetSettings = applyPetSettings;
exports.pet = pet;
exports.settingsPage = settingsPage;
exports.registerToolPkg = registerToolPkg;
exports.PACKAGE = "com.operit.deepseek_dafeiyu";
exports.PET_ROUTE = `toolpkg:${exports.PACKAGE}:ui:pet`;
exports.SETTINGS_ROUTE = `toolpkg:${exports.PACKAGE}:ui:settings`;
const WINDOW = "dafeiyu";
exports.DEFAULTS = { width: 208, x: 100, y: 100, animated: true, walk: false, clickThrough: false };
// All five source clips contain 241 frames at 24 fps. Keep this in sync with
// resources/animation-info.json when replacing clips.
exports.ACTION_DURATION_MS = 10042;
let configuration;
let resources;
function validateSettings(value) {
    for (const key of ["width", "x", "y"]) {
        if (!Number.isFinite(value[key]))
            throw new Error(`${key} 必须是有限数值`);
    }
    if (value.width < 128 || value.width > 320)
        throw new Error("桌宠宽度必须在 128–320 之间");
    if (Math.abs(value.x) > 100000 || Math.abs(value.y) > 100000)
        throw new Error("位置超出范围");
    for (const key of ["animated", "walk", "clickThrough"]) {
        if (typeof value[key] !== "boolean")
            throw new Error(`${key} 必须是布尔值`);
    }
    return { width: Math.round(value.width), x: Math.round(value.x), y: Math.round(value.y),
        animated: value.animated, walk: value.walk, clickThrough: value.clickThrough };
}
function config() {
    return configuration ?? (configuration = PluginConfig.use("dafeiyu", { settings: { ...exports.DEFAULTS } }));
}
async function sprites() {
    if (!resources) {
        resources = Promise.all(["idle", "happy", "eat", "walk", "talk", "still"].map(async (key) => [key, await ToolPkg.readResource(`dafeiyu_${key}`)]))
            .then(entries => Object.fromEntries(entries));
        resources.catch(() => { resources = undefined; });
    }
    return resources;
}
function dimensions(settings) {
    return { x: settings.x, y: settings.y, width: settings.width,
        height: Math.ceil(settings.width * 352 / 272) + 44,
        alwaysOnTop: true, clickThrough: settings.clickThrough, focusable: false, draggable: true };
}
/** Explicit user action; permission requests have their own button in settings. */
async function openPet(settings) {
    settings = validateSettings(settings);
    const caps = await ToolPkg.ui.floatingWindows.capabilities();
    if (!caps.supported)
        throw new Error("当前窗口宿主尚未就绪");
    if (!caps.permissionGranted)
        throw new Error("请先点击「申请系统悬浮权限」，授权后返回并重试");
    const images = await sprites();
    const current = await ToolPkg.ui.floatingWindows.list();
    if (current.some(window => window.id === WINDOW)) {
        const result = await applyPetSettings(settings);
        await ToolPkg.ui.floatingWindows.show(WINDOW);
        return result;
    }
    return ToolPkg.ui.floatingWindows.open({ id: WINDOW, route: exports.PET_ROUTE,
        state: { settings: { ...settings }, sprites: images }, options: dimensions(settings) });
}
/** Updates the live session and preserves its dragged position and animation state. */
async function applyPetSettings(settings) {
    const checked = validateSettings(settings);
    const { x, y, ...options } = dimensions(checked);
    return ToolPkg.ui.floatingWindows.update(WINDOW, options, { settings: { ...checked } });
}
/** Animated assets play in the host image decoder, without per-frame JS traffic. */
function pet(ctx) {
    const [settings] = ctx.useState("settings", exports.DEFAULTS);
    const [images] = ctx.useState("sprites", {});
    const [action, setAction] = ctx.useState("action", { name: "idle", sequence: 0 });
    const sequence = ctx.useRef("actionSequence", 0);
    const [bubble, setBubble] = ctx.useState("bubble", "单击挥手，双击喂饭，长按碎碎念");
    ctx.useEffect("actionReset", () => {
        const timer = setTimeout(() => { setAction({ name: "idle", sequence: action.sequence }); setBubble(""); }, action.name === "idle" ? 3200 : exports.ACTION_DURATION_MS);
        return () => clearTimeout(timer);
    }, [action.name, action.sequence]);
    function interact(name, text) {
        setAction({ name, sequence: ++sequence.current });
        setBubble(text);
    }
    const gesture = ctx.Modifier
        .tapGestures({ onTap: () => interact("happy", "你好呀！今天也要元气满满！"),
        onDoubleTap: () => interact("eat", "米饭！再来一碗！"),
        onLongPress: () => interact("talk", "才不是大肥鱼，是大蓝鲸！设置在工具箱里哦。") });
    const animation = !settings.animated ? "still" : action.name !== "idle" ? action.name : settings.walk ? "walk" : "idle";
    return ctx.UI.Column({ fillMaxSize: true, spacing: 0 }, [
        ctx.UI.Box({ height: 44, fillMaxWidth: true, contentAlignment: "center" }, bubble ? [
            ctx.UI.Box({ padding: 7, backgroundColor: "#F0F5FF" }, [
                ctx.UI.Text({ text: bubble, fontSize: 11, color: "#344878", maxLines: 2 }),
            ]),
        ] : []),
        ctx.UI.Image({ key: "dafeiyu-sprite", uri: images[animation] ?? "", fillMaxWidth: true, weight: 1,
            animationKey: `${animation}:${action.sequence}`,
            contentScale: "fit", contentDescription: "大肥鱼：拖动移动，单击挥手，双击喂饭，长按碎碎念", modifier: gesture }),
    ]);
}
function settingsPage(ctx) {
    const [settings, setSettings] = ctx.useState("settings", { ...exports.DEFAULTS });
    const [previewImages, setPreviewImages] = ctx.useState("previewImages", {});
    const [previewAction, setPreviewAction] = ctx.useState("previewAction", "idle");
    const previewSequence = ctx.useRef("previewSequence", 0);
    const [previewVersion, setPreviewVersion] = ctx.useState("previewVersion", 0);
    const [status, setStatus] = ctx.useState("status", "正在读取设置…");
    const [busy, setBusy] = ctx.useState("busy", false);
    const [ready, setReady] = ctx.useState("ready", false);
    const [scope, setScope] = ctx.useState("scope", "");
    const changes = ctx.useRef("changes", { latest: { ...exports.DEFAULTS }, dirty: false, running: false,
        disposed: false, timer: undefined });
    ctx.useEffect("releaseSettings", () => () => {
        changes.current.disposed = true;
        clearTimeout(changes.current.timer);
    }, []);
    async function flushChanges() {
        const pending = changes.current;
        if (pending.running || pending.disposed)
            return;
        pending.running = true;
        try {
            while (pending.dirty && !pending.disposed) {
                pending.dirty = false;
                const checked = validateSettings(pending.latest), store = await config();
                store.settings = checked;
                await PluginConfig.flush(store);
                const opened = (await ToolPkg.ui.floatingWindows.list()).some(window => window.id === WINDOW);
                if (opened)
                    await applyPetSettings(checked);
                if (!pending.dirty && !pending.disposed)
                    setStatus(opened ? "设置已保存并实时应用" : "设置已保存，打开后生效");
            }
        }
        catch (error) {
            if (!pending.disposed)
                setStatus(String(error));
        }
        finally {
            pending.running = false;
        }
    }
    function changeSettings(next, debounce = false) {
        const checked = validateSettings(next);
        if (checked.walk !== changes.current.latest.walk || checked.animated !== changes.current.latest.animated) {
            setPreviewAction(checked.walk ? "walk" : "idle");
            setPreviewVersion(++previewSequence.current);
        }
        changes.current.latest = checked;
        changes.current.dirty = true;
        setSettings(checked);
        clearTimeout(changes.current.timer);
        if (debounce)
            changes.current.timer = setTimeout(() => { void flushChanges(); }, 80);
        else
            void flushChanges();
    }
    const { UI } = ctx;
    async function describe() {
        const caps = await ToolPkg.ui.floatingWindows.capabilities();
        setScope(caps.scope === "system" ? "系统悬浮 · 可显示在其他应用上方" :
            caps.scope === "application" ? "应用内悬浮 · 离开 Operit 后不显示" : "当前宿主不可用");
        setStatus(caps.permissionGranted ? "已就绪。打开大肥鱼，拖动它试试看。" : "需要系统悬浮窗权限，请授权后返回。");
    }
    async function action(work, success) {
        if (busy)
            return;
        setBusy(true);
        try {
            await work();
            if (success)
                setStatus(success);
        }
        catch (error) {
            setStatus(String(error));
        }
        finally {
            setBusy(false);
        }
    }
    const toggle = (title, detail, key) => UI.Row({ fillMaxWidth: true, spacing: 12, verticalAlignment: "center" }, [
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
                setPreviewImages(await sprites());
                setPreviewAction(saved.walk ? "walk" : "idle");
                await describe();
                setReady(true);
            }
            catch (error) {
                setStatus(String(error));
            }
        },
    }, [
        UI.Text({ text: "大肥鱼桌宠", style: "headlineMedium" }),
        UI.Text({ text: "让这只吃白饭的小蓝鲸，住在你的桌面上。", style: "bodyLarge" }),
        UI.Card({ fillMaxWidth: true }, [UI.Column({ padding: 16, spacing: 8, horizontalAlignment: "center" }, [
                ...(previewImages.idle ? [UI.Image({ uri: previewImages[settings.animated ? previewAction : "still"],
                        animationKey: `preview:${previewAction}:${previewVersion}`, height: 190, contentScale: "fit",
                        contentDescription: "大肥鱼原生动作动画预览" })] : []),
                UI.FlowRow({ spacing: 8 }, [["idle", "待机"], ["happy", "挥手"], ["eat", "吃饭"],
                    ["walk", "踏步"], ["talk", "碎碎念"]].map(([name, label]) => UI.TextButton({ text: label, enabled: ready && settings.animated, onClick: () => {
                        setPreviewAction(name);
                        setPreviewVersion(++previewSequence.current);
                    } }))),
                UI.Text({ text: scope, style: "labelLarge" }),
                UI.Text({ text: status, style: "bodyMedium" }),
            ])]),
        UI.FlowRow({ spacing: 8 }, [
            UI.Button({ text: "保存并打开 / 应用设置", enabled: ready && !busy, onClick: () => action(async () => {
                    const checked = validateSettings(settings), store = await config();
                    store.settings = checked;
                    await PluginConfig.flush(store);
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
                toggle("播放动画", "眨眼转头、挥手、端碗吃饭和碎碎念口型；关闭后显示静态形象。", "animated"),
                toggle("行走展示", "待机时播放原地踏步动作，仍可自由拖动位置。", "walk"),
                toggle("鼠标 / 触摸穿透", "整窗输入透过大肥鱼。可从这个设置页直接关闭穿透。", "clickThrough"),
                UI.Text({ text: "单击挥手 · 双击喂饭 · 按住拖动 · 长按碎碎念", style: "bodySmall" }),
                UI.Text({ text: "碎碎念包含口型与表情动画，气泡为预设台词。", style: "bodySmall" }),
            ])]),
        UI.FlowRow({ spacing: 8 }, [
            UI.Button({ text: "隐藏", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.hide(WINDOW), "已隐藏") }),
            UI.Button({ text: "显示", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.show(WINDOW), "已显示") }),
            UI.Button({ text: "关闭", enabled: !busy, onClick: () => action(() => ToolPkg.ui.floatingWindows.close(WINDOW), "已关闭") }),
            UI.TextButton({ text: "恢复默认设置", enabled: !busy, onClick: () => changeSettings({ ...exports.DEFAULTS }) }),
        ]),
        UI.Text({ text: "DeepSeek 社区二创，非官方插件。动作素材：github.com/PC2005-cloud/dsh-pet，允许开源使用、禁止商用；形象遵循 CC BY-NC-SA 4.0。完整来源与授权见 THIRD_PARTY_NOTICES.md。", style: "bodySmall" }),
    ]);
}
function registerToolPkg() {
    ToolPkg.registerUiRoute({ id: "settings", route: exports.SETTINGS_ROUTE, runtime: "compose_dsl", screen: settingsPage,
        title: { zh: "大肥鱼桌宠", en: "Dafeiyu Pet" } });
    ToolPkg.registerUiRoute({ id: "pet", route: exports.PET_ROUTE, runtime: "compose_dsl", screen: pet,
        title: { zh: "大肥鱼", en: "Dafeiyu" } });
    ToolPkg.registerNavigationEntry({ id: "settings", route: exports.SETTINGS_ROUTE, surface: "toolbox", icon: "SmartToy",
        title: { zh: "大肥鱼桌宠", en: "Dafeiyu Pet" } });
    return true;
}
