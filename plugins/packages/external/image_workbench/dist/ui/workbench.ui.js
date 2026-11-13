"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/ui/workbench.ui.ts
var workbench_ui_exports = {};
__export(workbench_ui_exports, {
  default: () => Workbench
});
module.exports = __toCommonJS(workbench_ui_exports);

// src/catalog.ts
var PACKAGE_NAME = "image_workbench";
var PROVIDERS = [
  { id: "openai", title: "OpenAI", description: "OpenAI 与兼容图像接口", model: "gpt-image-1", apiKey: "OPENAI_API_KEY", endpointKey: "OPENAI_API_BASE_URL", endpoint: "https://api.openai.com", options: ["size"] },
  { id: "qwen", title: "通义千问", description: "百炼 / 通义万相，支持负面提示词", model: "qwen-image-plus", apiKey: "DASHSCOPE_API_KEY", endpointKey: "DASHSCOPE_API_BASE_URL", endpoint: "https://dashscope.aliyuncs.com", options: ["size", "count", "negative_prompt", "optimize_prompt", "watermark"] },
  { id: "zhipu", title: "智谱", description: "GLM Image 图像生成", model: "glm-image", apiKey: "ZHIPU_API_KEY", options: ["size"] },
  { id: "xai", title: "xAI / Grok", description: "图像生成，也可切换到视频", model: "grok-2-image-1212", apiKey: "XAI_API_KEY", options: ["size"] },
  { id: "minimax", title: "MiniMax", description: "文生图、参考图与批量生成", model: "image-01", apiKey: "MINIMAX_API_KEY", endpointKey: "MINIMAX_API_BASE_URL", endpoint: "https://api.minimaxi.com", options: ["size", "aspect_ratio", "count", "optimize_prompt", "watermark", "seed", "reference_images"] },
  { id: "nanobanana", title: "Nano Banana", description: "Grsai 接口，支持参考图与高分辨率", model: "nano-banana-pro", apiKey: "NANOBANANA_API_KEY", endpointKey: "NANOBANANA_API_BASE_URL", endpoint: "https://grsai.dakka.com.cn", options: ["aspect_ratio", "resolution", "reference_images"] }
];
function providerById(id) {
  const provider = PROVIDERS.find((item) => item.id === id);
  if (!provider) throw new Error(`未知生图方式：${id}`);
  return provider;
}
function validateCommon(input) {
  if (typeof input.prompt !== "string" || !input.prompt.trim()) throw new Error("请填写提示词。");
  if (input.model !== void 0 && (typeof input.model !== "string" || !input.model.trim())) throw new Error("模型名称不能为空。");
  if (input.file_name !== void 0 && (typeof input.file_name !== "string" || !input.file_name.trim() || /[\\/:*?"<>|]/.test(input.file_name) || [".", ".."].includes(input.file_name.trim()))) throw new Error("保存名称不能为空或包含目录与特殊字符。");
}
function validateImageRequest(input) {
  const provider = providerById(input.provider);
  validateCommon(input);
  const allowed = /* @__PURE__ */ new Set(["provider", "prompt", "model", "file_name", ...provider.options]);
  for (const [key, value] of Object.entries(input)) {
    if (value !== void 0 && !allowed.has(key)) throw new Error(`${provider.title} 不支持参数 ${key}。`);
  }
  for (const key of ["size", "aspect_ratio", "resolution", "negative_prompt"]) {
    if (input[key] !== void 0 && (typeof input[key] !== "string" || !input[key].trim())) throw new Error(`${key} 需要填写非空文字。`);
  }
  if (input.count !== void 0 && (!Number.isInteger(input.count) || input.count < 1 || input.count > (provider.id === "minimax" ? 9 : 4))) throw new Error("图片数量超出所选方式允许的范围。");
  if (input.seed !== void 0 && !Number.isInteger(input.seed)) throw new Error("随机种子必须为整数。");
  for (const key of ["optimize_prompt", "watermark"]) {
    if (input[key] !== void 0 && typeof input[key] !== "boolean") throw new Error(`${key} 必须是布尔值。`);
  }
  if (input.size !== void 0 && !/^\d+x\d+$/.test(input.size)) throw new Error("尺寸格式应为 1024x1024。");
  if (input.reference_images !== void 0 && (!Array.isArray(input.reference_images) || input.reference_images.some((uri) => typeof uri !== "string" || !uri.trim()))) throw new Error("参考图必须是非空 URL 或 VFS 路径的数组。");
  return provider;
}
function imageRequestFromFields(provider, prompt, fields) {
  const request = { provider, prompt };
  for (const [key, value] of Object.entries(fields)) {
    if (!value.trim()) continue;
    if (key === "count" || key === "seed") request[key] = Number(value);
    else if (key === "optimize_prompt" || key === "watermark") {
      if (value !== "true" && value !== "false") throw new Error(`${key} 请选择开启或关闭。`);
      request[key] = value === "true";
    } else if (key === "reference_images") request[key] = value.split("\n").map((uri) => uri.trim()).filter(Boolean);
    else request[key] = value.trim();
  }
  const typed = request;
  validateImageRequest(typed);
  return typed;
}

// src/ui/workbench.ui.ts
var LABELS = { model: "模型", size: "像素尺寸 · 例如 1024x1024", aspect_ratio: "画面比例 · 例如 16:9", resolution: "分辨率", count: "图片数量", negative_prompt: "负面提示词", optimize_prompt: "提示词优化", watermark: "水印", seed: "随机种子", reference_images: "参考图片 · URL 或文件路径，每行一张", file_name: "保存名称（可选）", duration: "视频时长（秒）", reference_image: "参考图片 · URL 或文件路径", reference_video: "参考视频 URL" };
function Workbench(ctx) {
  const { UI, Modifier: M } = ctx, colors = ctx.MaterialTheme.colorScheme;
  const [providerId, setProviderId] = ctx.useState("provider", "openai");
  const provider = providerById(providerId);
  const [mode, setMode] = ctx.useState("mode", "image");
  const [prompt, setPrompt] = ctx.useState("prompt", "");
  const [forms, setForms] = ctx.useState("forms", { ...Object.fromEntries(PROVIDERS.map((item) => [item.id, { model: item.model }])), xai_video: { model: "grok-imagine-video" } });
  const formKey = mode === "video" ? "xai_video" : provider.id, fields = forms[formKey];
  const [advanced, setAdvanced] = ctx.useState("advanced", false);
  const [settingsOpen, setSettingsOpen] = ctx.useState("settingsOpen", false);
  const [envValues, setEnvValues] = ctx.useState("envValues", {});
  const [settingsReady, setSettingsReady] = ctx.useState("settingsReady", false);
  const [busy, setBusy] = ctx.useState("busy", "");
  const lock = ctx.useRef("operationLock", false);
  const [notice, setNotice] = ctx.useState("notice", "");
  const [error, setError] = ctx.useState("error", "");
  const [history, setHistory] = ctx.useState("history", []);
  const historyRef = ctx.useRef("historyRef", history);
  const credentialKeys = [provider.apiKey, ...provider.endpointKey ? [provider.endpointKey] : [], ...provider.options.includes("reference_images") ? ["BEEIMG_API_KEY"] : []];
  ctx.useEffect("credentials", () => {
    let active = true;
    setSettingsReady(false);
    setError("");
    async function load() {
      try {
        const entries = await Promise.all(credentialKeys.map(async (key) => {
          const value = await Tools.SoftwareSettings.readEnvironmentVariable(PACKAGE_NAME, key);
          return [key, value.value ?? ""];
        }));
        if (active) {
          setEnvValues(Object.fromEntries(entries));
          setSettingsReady(true);
        }
      } catch (cause) {
        if (active) setError(String(cause));
      }
    }
    void load();
    return () => {
      active = false;
    };
  }, [provider.id]);
  function edit(key, value) {
    setForms({ ...forms, [formKey]: { ...fields, [key]: value } });
  }
  async function run(label, action) {
    if (lock.current) return;
    lock.current = true;
    setBusy(label);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      lock.current = false;
      setBusy("");
    }
  }
  async function saveSettings() {
    await run("正在保存配置…", async () => {
      for (const key of credentialKeys) await Tools.SoftwareSettings.writeEnvironmentVariable(PACKAGE_NAME, key, envValues[key].trim());
      setNotice(`${provider.title} 配置已保存。`);
    });
  }
  async function pickReference() {
    await run("正在选择参考图片…", async () => {
      const picked = await ctx.openFilePicker({ picker: "image", allowMultiple: mode === "image" });
      if (picked.cancelled) return;
      edit(mode === "image" ? "reference_images" : "reference_image", picked.files.map((file) => file.path).join("\n"));
    });
  }
  async function generate() {
    await run(mode === "image" ? "正在生成图片，请稍候…" : "正在生成视频，请稍候…", async () => {
      let request;
      if (mode === "image") request = { ...imageRequestFromFields(provider.id, prompt, fields) };
      else {
        const video = { provider: "xai", prompt };
        for (const [key, value] of Object.entries(fields)) {
          if (!value.trim()) continue;
          video[key] = key === "duration" ? Number(value) : value.trim();
        }
        if (video.duration !== void 0 && (!Number.isInteger(video.duration) || video.duration < 1 || video.duration > 15)) throw new Error("视频时长应为 1 到 15 秒的整数。");
        request = { ...video };
      }
      const response = await ctx.callTool(`${PACKAGE_NAME}:generate_${mode}`, request);
      if (!response.success || !response.data) throw new Error(response.message);
      const next = [response.data, ...historyRef.current].slice(0, 20);
      historyRef.current = next;
      setHistory(next);
      setNotice("生成完成，文件已保存。");
    });
  }
  function field(key) {
    if (key === "optimize_prompt" || key === "watermark") return UI.Column({ spacing: 6 }, [
      UI.Text({ text: LABELS[key], style: "labelLarge" }),
      UI.FlowRow({ spacing: 6, runSpacing: 6 }, [["", "默认"], ["true", "开启"], ["false", "关闭"]].map(([value, text]) => choice(text, fields[key] === value || fields[key] === void 0 && value === "", () => edit(key, value))))
    ]);
    return UI.TextField({
      label: LABELS[key],
      value: fields[key] ?? "",
      onValueChange: (value) => edit(key, value),
      singleLine: key !== "reference_images" && key !== "negative_prompt",
      minLines: key === "reference_images" ? 2 : 1,
      maxLines: 5,
      readOnly: !!busy,
      fillMaxWidth: true,
      ...key === "resolution" ? { supportingText: UI.Text({ text: mode === "video" ? "480p 或 720p" : "nano-banana-pro 支持 1K、2K、4K" }) } : {}
    });
  }
  function choice(text, selected, onClick) {
    return selected ? UI.Button({ text, enabled: !busy, onClick }) : UI.OutlinedButton({ enabled: !busy, onClick }, UI.Text({ text }));
  }
  function card(title, children) {
    return UI.Card({ fillMaxWidth: true }, UI.Column({ padding: 18, spacing: 14, fillMaxWidth: true, horizontalAlignment: "start" }, [UI.Text({ text: title, style: "titleMedium", fontWeight: "600" }), ...children]));
  }
  const options = mode === "video" ? ["aspect_ratio", "resolution", "duration", "reference_image", "reference_video"] : provider.options;
  const basic = options.filter((key) => ["size", "aspect_ratio", "resolution", "count", "reference_images", "reference_image", "reference_video", "duration"].includes(key));
  const extra = options.filter((key) => !basic.includes(key));
  const content = [
    UI.Row({ fillMaxWidth: true, verticalAlignment: "center" }, [UI.Icon({ name: "palette", size: 30, tint: colors.primary }), UI.Spacer({ width: 12 }), UI.Column({ weight: 1, horizontalAlignment: "start", spacing: 4 }, [UI.Text({ text: "生图工作台", style: "headlineSmall", fontWeight: "600" }), UI.Text({ text: "选择一种方式，把想象变成图片", style: "bodyMedium", color: colors.onSurfaceVariant })])]),
    card("生图方式", [
      UI.FlowRow({ spacing: 8, runSpacing: 8, fillMaxWidth: true }, PROVIDERS.map((item) => choice(item.title, item.id === provider.id, () => {
        setProviderId(item.id);
        setMode("image");
        setNotice("");
      }))),
      UI.Text({ text: provider.description, color: colors.onSurfaceVariant }),
      ...provider.id === "xai" ? [UI.FlowRow({ spacing: 8 }, ["image", "video"].map((value) => choice(value === "image" ? "生成图片" : "生成视频", mode === value, () => setMode(value))))] : [],
      UI.TextButton({ text: settingsOpen ? "收起接口配置" : "配置 API Key 与接口", enabled: !busy, onClick: () => setSettingsOpen(!settingsOpen) }),
      ...!settingsReady ? [UI.Text({ text: "正在读取配置…", style: "bodySmall" })] : !envValues[provider.apiKey]?.trim() ? [UI.Text({ text: "首次使用请先配置此方式的 API Key。", color: colors.primary })] : []
    ])
  ];
  if (settingsOpen && settingsReady) content.push(card("接口配置", [
    ...credentialKeys.map((key) => UI.TextField({ label: key === provider.apiKey ? `${provider.title} API Key` : key === "BEEIMG_API_KEY" ? "Beeimg API Key（本地参考图上传）" : "接口地址（可选）", value: envValues[key], onValueChange: (value) => setEnvValues({ ...envValues, [key]: value }), isPassword: key.endsWith("_API_KEY"), singleLine: true, fillMaxWidth: true, readOnly: !!busy, ...key === provider.endpointKey ? { placeholder: provider.endpoint } : {} })),
    UI.Button({ text: "保存配置", enabled: !busy, onClick: saveSettings })
  ]));
  content.push(card(mode === "image" ? "画面描述" : "视频描述", [
    UI.TextField({ label: "提示词", placeholder: "描述主体、场景、风格、光线和构图…", value: prompt, onValueChange: setPrompt, minLines: 4, maxLines: 10, fillMaxWidth: true, readOnly: !!busy }),
    field("model"),
    ...basic.map(field),
    ...provider.options.includes("reference_images") && mode === "image" || mode === "video" ? [UI.TextButton({ text: "选择参考图片", enabled: !busy, onClick: pickReference })] : [],
    ...mode === "image" && provider.options.includes("reference_images") ? [UI.Text({ text: "本地参考图会上传到 Beeimg，再提交给所选服务；需配置 Beeimg API Key。也可直接填写公网图片 URL。", style: "bodySmall", color: colors.onSurfaceVariant })] : [],
    UI.TextButton({ text: advanced ? "收起更多参数" : "更多参数", enabled: !busy, onClick: () => setAdvanced(!advanced) }),
    ...advanced ? [...extra.map(field), field("file_name")] : [],
    UI.Button({ text: busy || (mode === "image" ? "开始生图" : "开始生成视频"), enabled: !busy && !!prompt.trim() && settingsReady, onClick: generate, fillMaxWidth: true }),
    ...busy ? [UI.LinearProgressIndicator({ fillMaxWidth: true })] : []
  ]));
  if (error) content.push(UI.SelectionContainer({}, UI.Text({ text: error, color: colors.error })));
  if (notice) content.push(UI.Text({ text: notice, color: colors.primary }));
  if (history.length === 0) content.push(card("生成结果", [UI.Icon({ name: "image", size: 42, tint: colors.onSurfaceVariant }), UI.Text({ text: "生成的图片与视频会显示在这里。文件保存在插件目录，当前页面保留最近 20 次结果。", color: colors.onSurfaceVariant })]));
  for (const item of history) content.push(card(`${providerById(item.provider).title} · ${item.model}`, [
    UI.Text({ text: item.prompt, maxLines: 3, color: colors.onSurfaceVariant }),
    ...item.assets.map((asset) => UI.Column({ spacing: 8, fillMaxWidth: true }, [item.kind === "image" ? UI.Image({ uri: asset.uri, height: 320, fillMaxWidth: true, contentScale: "fit", contentDescription: item.prompt }) : UI.Markdown({ text: asset.markdown }), UI.SelectionContainer({}, UI.Text({ text: asset.uri, style: "bodySmall", color: colors.onSurfaceVariant }))])),
    UI.FlowRow({ spacing: 8, runSpacing: 8 }, [UI.TextButton({ text: "复用描述", enabled: !busy, onClick: () => setPrompt(item.prompt) }), UI.TextButton({ text: "复制结果引用", enabled: !busy, onClick: async () => {
      await run("正在复制…", async () => {
        await Tools.System.setClipboardText(item.markdown);
        setNotice("结果引用已复制。");
      });
    } })])
  ]));
  return UI.ScrollArea({ fillMaxSize: true }, UI.Box({ fillMaxWidth: true, contentAlignment: "topCenter", padding: 16 }, UI.Column({ modifier: M.widthIn(0, 1e3), fillMaxWidth: true, spacing: 16, horizontalAlignment: "start" }, content)));
}
