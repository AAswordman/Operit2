import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import { callAdmin } from "../admin.js";
import type { Mode, ModeContext } from "../mode.js";
import type { Settings } from "../api.js";

const emotions: Record<string, string> = { crying: "哭泣", like_you: "喜欢你", happy: "开心", surprised: "惊讶", miss_you: "想你", speechless: "无语", angry: "生气", confused: "困惑", sad: "难过" };

/** Recreates the role-bound Operit1 settings with native cards and direct manipulation. */
export default function screen(ctx: ComposeDslContext): ComposeNode {
  const [context, setContext] = ctx.useState<ModeContext | null>("context", null);
  const current = ctx.useRef<ModeContext | null>("current", null);
  const saves = ctx.useRef<Promise<void>>("saves", Promise.resolve());
  const revision = ctx.useRef("revision", 0);
  const [status, setStatus] = ctx.useState("status", "");
  const [error, setError] = ctx.useState("error", "");
  const [page, setPage] = ctx.useState("page", "settings");
  const [category, setCategory] = ctx.useState("category", "happy");
  const [imageUrl, setImageUrl] = ctx.useState("imageUrl", "");
  const [previews, setPreviews] = ctx.useState<Record<string, string[]>>("previews", {});
  const colors = ctx.MaterialTheme.colorScheme;

  /** Loads the current character binding through the authoritative settings channel. */
  async function load(): Promise<void> {
    try { const value = await callAdmin({ action: "mode.get" }); current.current = value; setContext(value); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }

  /** Saves complete edits in order while preserving immediate local form interaction. */
  function update(changes: Partial<Mode>, settings: Partial<Settings> = {}): void {
    const previous = current.current;
    if (previous === null) throw new Error("Waifu settings are not loaded");
    const mode = { ...previous.mode, ...changes, settings: { ...previous.mode.settings, ...settings } };
    const next = { ...previous, mode };
    current.current = next; setContext(next); setError(""); setStatus("正在保存…");
    const version = ++revision.current;
    saves.current = saves.current.then(async () => {
      await callAdmin({ action: "mode.set", selection: next.target.selection, mode });
      if (version === revision.current) setStatus("已自动保存");
    }).catch(failure => { setStatus(""); setError(failure instanceof Error ? failure.message : String(failure)); });
  }

  /** Resolves image previews through packaged resources without exposing resource identifiers. */
  async function showEmojis(): Promise<void> {
    if (current.current === null) return;
    try { setPreviews(await callAdmin({ action: "emoji.previews", emojis: current.current.mode.settings.emojis })); setPage("emojis"); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }

  /** Replaces one category while keeping the same role-bound image configuration. */
  async function imagesChanged(images: string[]): Promise<void> {
    if (current.current === null) return;
    const emojis = { ...current.current.mode.settings.emojis };
    if (images.length === 0) delete emojis[category]; else emojis[category] = images;
    update({}, { emojis });
    setPreviews(await callAdmin({ action: "emoji.previews", emojis }));
  }

  /** Imports selected image files into plugin-owned durable Host storage. */
  async function importImages(): Promise<void> {
    try {
      const picked = await ctx.openFilePicker({ picker: "image", allowMultiple: true, mimeTypes: ["image/*"] });
      if (picked.cancelled) return;
      const files = picked.files.map(file => {
        if (file.name === undefined) throw new Error("文件选择器没有返回图片文件名");
        return { path: file.path, name: file.name };
      });
      const paths = await callAdmin({ action: "emoji.import", files });
      const existing = current.current!.mode.settings.emojis[category];
      await imagesChanged(existing === undefined ? paths : [...existing, ...paths]);
    } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
  }

  /** Creates a Material settings card with a consistent icon, title and body. */
  function card(icon: string, title: string, children: ComposeNode[], highlight = false): ComposeNode {
    return ctx.UI.Card({ fillMaxWidth: true, containerColor: highlight ? colors.secondaryContainer : colors.surfaceContainerLow, shape: { cornerRadius: 16 } },
      ctx.UI.Column({ padding: 16, spacing: 12, fillMaxWidth: true }, [
        ctx.UI.Row({ spacing: 12, verticalAlignment: "center" }, [ctx.UI.Icon({ name: icon, size: 22, tint: colors.primary }), ctx.UI.Text({ text: title, style: "titleMedium" })]), ...children,
      ]));
  }

  /** Renders explanatory text using the theme's secondary foreground. */
  function description(text: string): ComposeNode { return ctx.UI.Text({ text, style: "bodyMedium", color: colors.onSurfaceVariant }); }

  /** Renders a full-width native switch with its original explanatory copy. */
  function toggle(title: string, text: string, checked: boolean, onCheckedChange: (value: boolean) => void, enabled = true): ComposeNode {
    return ctx.UI.Row({ fillMaxWidth: true, horizontalArrangement: "spaceBetween", verticalAlignment: "center", spacing: 16 }, [
      ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: title, style: "titleMedium" }), description(text)]),
      ctx.UI.Switch({ checked, onCheckedChange, enabled }),
    ]);
  }

  if (context === null) return ctx.UI.Column({ fillMaxSize: true, padding: 24, spacing: 16, onLoad: load }, [description("正在读取当前角色的 Waifu 设置…"), ctx.UI.Text({ text: error, color: colors.error })]);
  const { mode, target } = context, settings = mode.settings;
  const feedback = [ctx.UI.Text({ text: error, color: colors.error }), description(status)];

  if (page === "emojis") {
    const images = settings.emojis[category];
    const sources = images === undefined ? [] : images;
    const previewImages = previews[category];
    return ctx.UI.LazyColumn({ fillMaxSize: true, padding: 16, spacing: 12 }, [
      ctx.UI.Row({ spacing: 8, verticalAlignment: "center" }, [ctx.UI.IconButton({ icon: "ArrowBack", onClick: () => setPage("settings") }), ctx.UI.Text({ text: "管理自定义表情包", style: "headlineSmall" })]),
      description(`为「${target.name}」管理表情图片。AI 会根据情绪从对应分类中选取图片。`),
      ctx.UI.FlowRow({ spacing: 8, fillMaxWidth: true }, Object.entries(emotions).map(([key, title]) => (key === category ? ctx.UI.Button : ctx.UI.OutlinedButton)({ text: title, onClick: () => setCategory(key) }))),
      card("EmojiEmotions", `${emotions[category]} · ${sources.length} 张`, [
        ctx.UI.Button({ text: "从设备导入图片", onClick: importImages }),
        ctx.UI.TextField({ label: "图片网址", value: imageUrl, onValueChange: setImageUrl, singleLine: true, fillMaxWidth: true }),
        ctx.UI.Button({ text: "添加图片网址", enabled: imageUrl.trim() !== "", onClick: async () => {
          try {
            const url = imageUrl.trim();
            if (!/^https?:\/\/[^\s<>]+$/u.test(url)) throw new Error("请输入 HTTP 或 HTTPS 图片网址");
            await imagesChanged([...sources, url]); setImageUrl("");
          } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
        } }),
      ]),
      ctx.UI.FlowRow({ spacing: 12, fillMaxWidth: true }, sources.map((_source, index) => ctx.UI.Card({ width: 140 }, ctx.UI.Column({ padding: 8, spacing: 4 }, [
        ctx.UI.Image({ uri: previewImages![index], width: 124, height: 124, contentScale: "fit", contentDescription: `${emotions[category]}表情 ${index + 1}` }),
        ctx.UI.TextButton({ text: "移除", onClick: () => imagesChanged(sources.filter((_image, position) => position !== index)) }),
      ])))), ...feedback,
    ]);
  }

  return ctx.UI.LazyColumn({ fillMaxSize: true, padding: 16, spacing: 12 }, [
    card("Face", "Waifu 模式", [description("将 AI 回复按句子分成独立气泡，配合打字节奏和表情包，让对话更自然。")], true),
    card("Link", "绑定到角色卡", [ctx.UI.Text({ text: target.name, style: "titleMedium", color: colors.primary }), description("设置跟随当前角色卡或角色组，在普通聊天中生效。")], true),
    card("ChatBubbleOutline", "聊天模式", [toggle("启用 Waifu 模式", "AI 回复按句子发送，每句显示为独立气泡。", mode.enabled, enabled => update({ enabled }))]),
    card("Forum", "合并发送", [toggle("合并连续消息", "连续发送短消息时，停顿后合成一条消息交给 AI。", mode.mergeSend, mergeSend => update({ mergeSend })),
      ...(mode.mergeSend ? [description(`等待 ${(mode.mergeDelayMs / 1000).toFixed(1)} 秒后发送`), ctx.UI.Slider({ min: 500, max: 10000, divisions: 95, value: mode.mergeDelayMs, label: `${mode.mergeDelayMs} ms`, onValueChange: value => update({ mergeDelayMs: Math.round(value) }) })] : []),
    ]),
    card("Speed", "打字速度", [description(`每秒约 ${(1000 / settings.charDelayMs).toFixed(1)} 个字符`),
      ctx.UI.Row({ verticalAlignment: "center", spacing: 8 }, [description("快"), ctx.UI.Slider({ weight: 1, min: 200, max: 1000, divisions: 40, value: settings.charDelayMs, label: `${settings.charDelayMs} ms`, onValueChange: value => update({}, { charDelayMs: Math.round(value) }) }), description("慢")]),
      description(`每字符延迟 ${settings.charDelayMs} 毫秒。首句立即显示，后续句子按长度等待，单句最多 3 秒。`),
    ]),
    card("TextFields", "移除句末标点", [toggle("移除标点", "去掉每句末尾的句号、问号和感叹号，保留链接与代码。", settings.removePunctuation, removePunctuation => update({}, { removePunctuation }))]),
    card("EditNote", "自定义 Waifu 提示词", [description("自定义 Waifu 模式附加提示词。"), ctx.UI.TextField({ label: "附加提示词", value: settings.customPrompt, onValueChange: customPrompt => update({}, { customPrompt }), singleLine: false, minLines: 4, maxLines: 8, fillMaxWidth: true }), description("这段内容会作为 Waifu 模式的额外规则拼接到系统提示词中。")]),
    card("EmojiEmotions", "表情包", [toggle("启用表情包", "在对话中根据情绪插入表情图片，增强表达效果。", settings.enableEmoticons, enableEmoticons => update({}, { enableEmoticons })), description(`可用情绪：${Object.entries(emotions).filter(([key]) => settings.emojis[key] !== undefined).map(([, title]) => title).join("、")}`)]),
    ctx.UI.Card({ fillMaxWidth: true, modifier: ctx.Modifier.clickable(showEmojis) }, ctx.UI.Row({ padding: 16, spacing: 12, verticalAlignment: "center" }, [ctx.UI.Icon({ name: "PhotoLibrary", tint: colors.primary }), ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: "管理自定义表情包", style: "titleMedium" }), description("预览、导入或移除各情绪分类的图片")]), ctx.UI.Icon({ name: "ChevronRight" })])),
    card("PhotoCamera", "自拍", [toggle("启用自拍", "允许 AI 在你请求自拍时使用已配置的绘图工具。", settings.enableSelfie, enableSelfie => update({}, { enableSelfie })),
      ...(settings.enableSelfie ? [ctx.UI.TextField({ label: "角色外貌关键词", value: settings.selfiePrompt, onValueChange: selfiePrompt => update({}, { selfiePrompt }), singleLine: false, minLines: 4, maxLines: 8, fillMaxWidth: true }), description("这些关键词用于保持自拍中的角色外貌一致。绘图服务需在工具中配置。")] : []),
    ]),
    card("Info", "模式说明", [description("普通模式一次显示完整回复；Waifu 模式按句子显示独立气泡。\n智能延迟根据句子长度计算，连续输入可在停顿后合并发送。\n情绪标签自动转为表情图片，思考和工具结果保留各自的展示。\n设置自动保存，下一次回复生效。")]),
    ...feedback,
  ]);
}
