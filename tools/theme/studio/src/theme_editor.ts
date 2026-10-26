import themeSchema from "../../../../core/crates/plugin/theme/schema/theme-v1.schema.json";
import fixture from "../../../../apps/flutter/packages/operit_ui/assets/fixtures/theme-v1.json";
import { closeStudioColorPopover, openStudioColorPopover } from "./color_popover";
import {
  PALETTE_SEED_PRESETS,
  extractSeedColorFromImageUrl,
  generateDualModePaletteFromSeed,
} from "./palette_generator";

type JsonObject = Record<string, unknown>;

export interface ThemeInspectorSection {
  label: string;
  path: string[];
  paths?: string[][];
  open?: boolean;
}

export interface ThemeInspectorTarget {
  id?: string;
  title: string;
  badge?: string;
  description: string;
  sections: ThemeInspectorSection[];
}

export const THEME_INSPECTOR_TARGETS: Readonly<Record<string, ThemeInspectorTarget>> = {
  background: {
    id: "background",
    title: "画布背景",
    badge: "Canvas Backdrop",
    description: "控制 Flutter 根画布的背景贴图、契合方式、不透明度与高斯模糊。",
    sections: [{ label: "图片", path: ["background"], open: true }],
  },
  "navigation.sidebar": {
    id: "navigation.sidebar",
    title: "导航侧栏",
    badge: "Navigation Rail",
    description: "侧边会话树与底部工具栏的 Material 表面色彩及毛玻璃穿透状态。",
    sections: [
      { label: "透明表面", path: ["surfaces"], open: true },
    ],
  },
  "chat.topBar": {
    id: "chat.topBar",
    title: "对话顶部栏",
    badge: "Top App Bar",
    description: "会话标题栏与右侧工具区的表面透明度及主题色阶。",
    sections: [
      { label: "透明表面", path: ["surfaces"], open: true },
    ],
  },
  "chat.userMessage": {
    id: "chat.userMessage",
    title: "用户消息气泡",
    badge: "User Message · Right",
    description: "定制右侧用户消息的气泡填充色、文字色、圆角、边距、PNG/SVG 九宫格贴图与字体。",
    sections: [
      { label: "气泡", path: ["chat", "user"], open: true },
      {
        label: "布局",
        path: ["chat"],
        paths: [
          ["chat", "style"],
          ["chat", "wideLayout"],
          ["chat", "showAvatar"],
          ["chat", "avatarShape"],
          ["chat", "avatarCornerRadius"],
        ],
        open: true,
      },
      {
        label: "字体",
        path: ["typography"],
        paths: [
          ["typography", "user"],
          ["typography", "global"],
        ],
        open: true,
      },
    ],
  },
  "chat.aiMessage": {
    id: "chat.aiMessage",
    title: "AI 消息气泡",
    badge: "Assistant Message · Left",
    description: "定制左侧 AI 回复消息的气泡填充色、文字色、圆角、边距、PNG/SVG 九宫格贴图与字体。",
    sections: [
      { label: "气泡", path: ["chat", "ai"], open: true },
      {
        label: "布局",
        path: ["chat"],
        paths: [
          ["chat", "style"],
          ["chat", "wideLayout"],
          ["chat", "showAvatar"],
          ["chat", "avatarShape"],
          ["chat", "avatarCornerRadius"],
        ],
        open: true,
      },
      {
        label: "字体",
        path: ["typography"],
        paths: [
          ["typography", "ai"],
          ["typography", "global"],
        ],
        open: true,
      },
    ],
  },
  "chat.composer": {
    id: "chat.composer",
    title: "消息输入框",
    badge: "Chat Composer",
    description: "底部输入容器的形态、九宫格贴图、右侧 5 个动作按钮贴图与尺寸调节。",
    sections: [
      { label: "外观", path: ["input"], open: true },
      { label: "透明表面", path: ["surfaces"], open: true },
      { label: "全局字体", path: ["typography", "global"], open: false },
    ],
  },
  typography: {
    id: "typography",
    title: "字体",
    badge: "Typography",
    description: "配置全局字体及用户/AI 消息专属 TTF 字体映射。",
    sections: [{ label: "字体", path: ["typography"], open: true }],
  },
  palette: {
    id: "palette",
    title: "调色板",
    badge: "M3 Color System",
    description: "亮色与暗色各 46 个语义色彩角色，支持从主色或背景图一键生成。",
    sections: [{ label: "颜色", path: ["palette"], open: true }],
  },
};

type SchemaNode = {
  $ref?: string;
  type?: string | string[];
  properties?: Record<string, SchemaNode>;
  enum?: string[];
  minimum?: number;
  maximum?: number;
  default?: unknown;
  pattern?: string;
};

const schema = themeSchema as unknown as SchemaNode & { $defs: Record<string, SchemaNode> };

interface FieldMeta {
  label: string;
  hint?: string;
  unit?: string;
  enumLabels?: Record<string, string>;
}

const INPUT_ACTION_SLOTS: ReadonlyArray<{
  assetKey: string;
  sizeKey: string;
  title: string;
  glyph: string;
  subtitle: string;
  minSize: number;
  maxSize: number;
  defaultSize: number;
}> = [
  {
    assetKey: "settingsAsset",
    sizeKey: "settingsAssetSize",
    title: "设置按钮",
    glyph: "⚙️",
    subtitle: "输入栏右侧设置按钮图标",
    minSize: 12,
    maxSize: 44,
    defaultSize: 18,
  },
  {
    assetKey: "attachmentAsset",
    sizeKey: "attachmentAssetSize",
    title: "添加附件",
    glyph: "➕",
    subtitle: "输入栏右侧添加附件图标",
    minSize: 12,
    maxSize: 44,
    defaultSize: 18,
  },
  {
    assetKey: "micAsset",
    sizeKey: "micAssetSize",
    title: "语音输入态",
    glyph: "🎙️",
    subtitle: "主按钮默认空输入时的语音状态",
    minSize: 12,
    maxSize: 48,
    defaultSize: 26,
  },
  {
    assetKey: "sendAsset",
    sizeKey: "sendAssetSize",
    title: "发送消息态",
    glyph: "⬆️",
    subtitle: "主按钮就绪可发送时的状态",
    minSize: 12,
    maxSize: 48,
    defaultSize: 26,
  },
  {
    assetKey: "stopAsset",
    sizeKey: "stopAssetSize",
    title: "停止生成态",
    glyph: "⏹️",
    subtitle: "主按钮在模型输出中时的停止状态",
    minSize: 12,
    maxSize: 48,
    defaultSize: 26,
  },
];


const LEGACY_SINGLE_COLOR_PATHS = new Set([
  "chat.cursorUserColor",
  "chat.user.color",
  "chat.user.textColor",
  "chat.ai.color",
  "chat.ai.textColor",
  "input.color",
]);

const INPUT_ACTION_KEY_SET = new Set<string>(
  INPUT_ACTION_SLOTS.flatMap(slot => [slot.assetKey, slot.sizeKey]),
);

const FIELD_META: Readonly<Record<string, FieldMeta>> = {
  "background.asset": { label: "背景图片", hint: "支持 PNG / JPEG / 静态 SVG，留空则使用纯色表面" },
  "background.fit": {
    label: "填充",
    enumLabels: {
      cover: "裁切铺满 Cover",
      contain: "完整包含 Contain",
      fill: "拉伸填满 Fill",
      fitWidth: "定宽自适应",
      fitHeight: "定高自适应",
      none: "原始尺寸",
      scaleDown: "仅缩小居中",
    },
  },
  "background.alignment": {
    label: "对齐",
    enumLabels: {
      center: "正中 Center",
      topLeft: "左上 TopLeft",
      topCenter: "顶中 TopCenter",
      topRight: "右上 TopRight",
      centerLeft: "左中 CenterLeft",
      centerRight: "右中 CenterRight",
      bottomLeft: "左下 BottomLeft",
      bottomCenter: "底中 BottomCenter",
      bottomRight: "右下 BottomRight",
    },
  },
  "background.opacity": { label: "不透明度", hint: "0 为完全透明，1 为原始不透明度" },
  "background.blur": { label: "模糊", hint: "对背景图层施加实时模糊滤镜" },
  "background.blur.enabled": { label: "启用模糊" },
  "background.blur.radius": { label: "模糊半径", unit: "px" },
  "surfaces.transparent": { label: "全局透明表面", hint: "开启后顶栏、侧栏与输入框透出底层画布背景" },
  "chat.style": {
    label: "布局",
    enumLabels: {
      bubble: "经典气泡 Bubble",
      cursor: "极简流式 Cursor",
    },
  },
  "chat.wideLayout": { label: "宽屏", hint: "扩展对话流最大内容宽度（700px → 1040px）" },
  "chat.showAvatar": { label: "头像", hint: "在用户与 AI 消息旁展示头像徽章" },
  "chat.avatarShape": {
    label: "头像形状",
    enumLabels: {
      circle: "正圆 Circle",
      square: "圆角方形 Square",
    },
  },
  "chat.avatarCornerRadius": { label: "头像圆角", unit: "px" },
  "chat.cursorUserColor": { label: "Cursor 模式用户消息底色", hint: "仅在极简流式 Cursor 模式下覆盖用户提问卡片底色" },
  "chat.user": { label: "用户消息气泡 (右侧)", hint: "定制用户发送消息的色彩、贴图与内边距" },
  "chat.ai": { label: "AI 消息气泡 (左侧)", hint: "定制助手回复消息的色彩、贴图与内边距" },
  "chat.user.color": { label: "背景" },
  "chat.user.textColor": { label: "文字" },
  "chat.user.roundedCorners": { label: "圆角" },
  "chat.user.padding": { label: "内边距" },
  "chat.user.padding.left": { label: "左内边距", unit: "px" },
  "chat.user.padding.right": { label: "右内边距", unit: "px" },
  "chat.user.image": { label: "用户气泡背景贴图 / 九宫格", hint: "支持 PNG/JPEG/SVG 贴图与九宫格可拉伸区域定义" },
  "chat.ai.color": { label: "背景" },
  "chat.ai.textColor": { label: "文字" },
  "chat.ai.roundedCorners": { label: "圆角" },
  "chat.ai.padding": { label: "内边距" },
  "chat.ai.padding.left": { label: "左内边距", unit: "px" },
  "chat.ai.padding.right": { label: "右内边距", unit: "px" },
  "chat.ai.image": { label: "AI 气泡背景贴图 / 九宫格", hint: "支持 PNG/JPEG/SVG 贴图与九宫格可拉伸区域定义" },
  "input.style": {
    label: "形状",
    enumLabels: {
      agent: "Agent 胶囊工作台",
      classic: "经典通栏输入框",
    },
  },
  "input.floating": { label: "悬浮", hint: "为输入框赋予独立悬浮层级与底部留白" },
  "input.color": { label: "背景", hint: "覆盖底部输入容器底色（留空则随调色板自动派生）" },
  "input.roundedCorners": { label: "圆角", hint: "像素风或直角边框贴图建议关闭此项以完整展示四角" },
  "input.image": { label: "输入框背景贴图 / 九宫格", hint: "支持 PNG/JPEG/SVG 无损贴图与九宫格可拉伸区域定义" },
  "input.settingsAsset": { label: "设置按钮素材 (⚙️)", hint: "替换输入框右侧「设置」按钮图标" },
  "input.settingsAssetSize": { label: "设置按钮素材尺寸", unit: "px" },
  "input.attachmentAsset": { label: "添加附件按钮素材 (➕)", hint: "替换输入框右侧「添加附件」按钮图标" },
  "input.attachmentAssetSize": { label: "添加附件素材尺寸", unit: "px" },
  "input.micAsset": { label: "语音输入状态素材 (🎙️)", hint: "替换主按钮「语音输入」状态素材" },
  "input.micAssetSize": { label: "语音输入素材尺寸", unit: "px" },
  "input.sendAsset": { label: "发送状态素材 (⬆️)", hint: "替换主按钮「发送」状态素材" },
  "input.sendAssetSize": { label: "发送状态素材尺寸", unit: "px" },
  "input.stopAsset": { label: "停止状态素材 (⏹️)", hint: "替换主按钮「停止」状态素材" },
  "input.stopAssetSize": { label: "停止状态素材尺寸", unit: "px" },
  "typography.global": { label: "全局 UI 与正文基准字体", hint: "默认使用内置思源黑体或 Cascadia Mono，也可上传 TTF" },
  "typography.user": { label: "用户消息独立字体", hint: "留空则自动继承全局字体" },
  "typography.ai": { label: "AI 消息独立字体", hint: "留空则自动继承全局字体" },
};



export interface QuickColorSwatch {
  label: string;
  hex8: string;
  isTransparent?: boolean;
}

export const QUICK_COLOR_SWATCHES: ReadonlyArray<QuickColorSwatch> = [
  { label: "纯白 #FFFFFF", hex8: "#FFFFFFFF" },
  { label: "柔白 #F1F5F9", hex8: "#F1F5F9FF" },
  { label: "浅灰 #CBD5E1", hex8: "#CBD5E1FF" },
  { label: "中灰 #64748B", hex8: "#64748BFF" },
  { label: "深灰 #1E293B", hex8: "#1E293BFF" },
  { label: "暗夜 #111318", hex8: "#111318FF" },
  { label: "纯黑 #000000", hex8: "#000000FF" },
  { label: "透明 0%", hex8: "#00000000", isTransparent: true },
  { label: "湛蓝 #3B82F6", hex8: "#3B82F6FF" },
  { label: "翡翠 #10B981", hex8: "#10B981FF" },
  { label: "琥珀 #F59E0B", hex8: "#F59E0BFF" },
  { label: "紫晶 #8B5CF6", hex8: "#8B5CF6FF" },
];

export interface ComponentDualPreset {
  label: string;
  applyRole: (token: string) => { light: string; dark: string };
}

function isAccentFillToken(token: string): boolean {
  return token === "primary" || token === "secondary" || token === "surfaceTint";
}

function isOnAccentToken(token: string): boolean {
  return token === "onPrimary" || token === "onTertiary";
}

function isTextOrIconToken(token: string): boolean {
  return token.startsWith("on") || token === "tertiary";
}

function isContainerSurfaceToken(token: string): boolean {
  return token.includes("Container") || token === "primaryFixed" || token === "secondaryFixed";
}

function isBorderToken(token: string): boolean {
  return token.startsWith("outline") || token === "tertiaryFixedDim";
}

export const COMPONENT_DUAL_PRESETS: ReadonlyArray<ComponentDualPreset> = [
  {
    label: "黑白",
    applyRole: token => {
      if (token === "error") return { light: "#BA1A1AFF", dark: "#FFB4ABFF" };
      if (isOnAccentToken(token)) return { light: "#FFFFFFFF", dark: "#000000FF" };
      if (isAccentFillToken(token)) return { light: "#111827FF", dark: "#F8FAFCFF" };
      if (isTextOrIconToken(token)) return { light: "#090D16FF", dark: "#FFFFFFFF" };
      if (isBorderToken(token)) return { light: "#CBD5E1FF", dark: "#334155FF" };
      if (isContainerSurfaceToken(token)) return { light: "#F1F5F9FF", dark: "#1E293BFF" };
      return { light: "#FFFFFFFF", dark: "#000000FF" };
    },
  },
  {
    label: "灰阶",
    applyRole: token => {
      if (token === "error") return { light: "#BA1A1AFF", dark: "#FFB4ABFF" };
      if (isOnAccentToken(token)) return { light: "#FFFFFFFF", dark: "#002E5AFF" };
      if (isAccentFillToken(token)) return { light: "#2563EBFF", dark: "#93C5FDFF" };
      if (isTextOrIconToken(token)) return { light: "#1E293BFF", dark: "#E2E8F0FF" };
      if (isBorderToken(token)) return { light: "#CBD5E1FF", dark: "#334155FF" };
      if (isContainerSurfaceToken(token)) return { light: "#E2E8F0FF", dark: "#1E293BFF" };
      return { light: "#F8FAFCFF", dark: "#0F172AFF" };
    },
  },
  {
    label: "暖灰",
    applyRole: token => {
      if (token === "error") return { light: "#BA1A1AFF", dark: "#FFB4ABFF" };
      if (isOnAccentToken(token)) return { light: "#FFFDF9FF", dark: "#291805FF" };
      if (isAccentFillToken(token)) return { light: "#B45309FF", dark: "#FBBF24FF" };
      if (isTextOrIconToken(token)) return { light: "#2E2012FF", dark: "#FDE68AFF" };
      if (isBorderToken(token)) return { light: "#D6C7B2FF", dark: "#453424FF" };
      if (isContainerSurfaceToken(token)) return { light: "#EFE6D5FF", dark: "#292018FF" };
      return { light: "#FAF6EEFF", dark: "#17120EFF" };
    },
  },
  {
    label: "蓝色",
    applyRole: token => {
      if (token === "error") return { light: "#BA1A1AFF", dark: "#FFB4ABFF" };
      if (isOnAccentToken(token)) return { light: "#FFFFFFFF", dark: "#042F2EFF" };
      if (isAccentFillToken(token)) return { light: "#0284C7FF", dark: "#38BDF8FF" };
      if (isTextOrIconToken(token)) return { light: "#0C4A6EFF", dark: "#E0F2FEFF" };
      if (isBorderToken(token)) return { light: "#BAE6FDFF", dark: "#1E3A5FFF" };
      if (isContainerSurfaceToken(token)) return { light: "#E0F2FEFF", dark: "#162C46FF" };
      return { light: "#F0F9FFFF", dark: "#081424FF" };
    },
  },
];

export interface ComponentColorRoleSpec {
  token: string;
  partLabel: string;
  usageHint: string;
}

export interface ComponentColorOverrideSpec {
  path: string[];
  label: string;
  fallbackLightToken: string;
}

export const COMPONENT_COLOR_MAP: Readonly<
  Record<
    string,
    {
      title: string;
      subtitle: string;
      overrides?: ComponentColorOverrideSpec[];
      roles: ComponentColorRoleSpec[];
    }
  >
> = {
  "navigation.sidebar": {
    title: "侧栏",
    subtitle: "独立控制左侧会话抽屉在「☀️ 亮色」与「🌙 暗色」下的文字、图标与选中背景；表面配色角色同时用于其他组件",
    roles: [
      {
        token: "onSurface",
        partLabel: "标题",
        usageHint: "左上角 Operit 标题、工作区与会话分组名称文字颜色",
      },
      {
        token: "onSurfaceVariant",
        partLabel: "文字与图标",
        usageHint: "未选中会话标题、搜索按钮、底部「包管理 / 设置」文字与图标颜色",
      },
      {
        token: "secondaryContainer",
        partLabel: "选中背景",
        usageHint: "正在查看的会话胶囊（如 New Chat）的高亮衬底色",
      },
      {
        token: "onSecondaryContainer",
        partLabel: "选中文字",
        usageHint: "选中会话行内的标题文字、拖拽手柄与更多图标颜色",
      },
      {
        token: "surface",
        partLabel: "背景",
        usageHint: "侧边栏根容器底色（开启透明表面时按低透明度叠色）",
      },
      {
        token: "surfaceContainerLow",
        partLabel: "按钮背景",
        usageHint: "顶部新建对话按钮栏、底部包管理/设置/亮暗切换按钮背景色",
      },
      {
        token: "primary",
        partLabel: "强调色",
        usageHint: "选中会话左侧竖向指示条、顶部开关高亮强调色",
      },
      {
        token: "outlineVariant",
        partLabel: "分隔线",
        usageHint: "分组下的竖向树状连接线、底部按钮细边框颜色",
      },
    ],
  },
  "chat.topBar": {
    title: "顶栏",
    subtitle: "独立控制顶部会话标题栏在「☀️ 亮色」与「🌙 暗色」下的标题文字、折叠按钮与背景",
    roles: [
      {
        token: "onSurface",
        partLabel: "标题与图标",
        usageHint: "会话标题（Operit - New Chat）、左侧收起侧栏图标、右侧工作区图标颜色",
      },
      {
        token: "surface",
        partLabel: "背景",
        usageHint: "未开启透明玻璃或无背景图时的顶部栏纯色背景",
      },
      {
        token: "onSurface",
        partLabel: "辅助文字",
        usageHint: "顶栏辅助徽章与次级图标颜色",
      },
    ],
  },
  "chat.userMessage": {
    title: "用户消息",
    subtitle: "独立为「☀️ 亮色」与「🌙 暗色」声明右侧用户消息的文字色与背景色",
    roles: [
      {
        token: "onPrimaryContainer",
        partLabel: "文字",
        usageHint: "右侧用户气泡内的聊天文本颜色",
      },
      {
        token: "primaryContainer",
        partLabel: "背景",
        usageHint: "未使用九宫格贴图时的背景",
      },
      {
        token: "primary",
        partLabel: "头像背景",
        usageHint: "右侧用户头像圆框/方框主题强调色",
      },
      {
        token: "onPrimary",
        partLabel: "头像图标",
        usageHint: "用户头像内部默认人像图标颜色",
      },
    ],
  },
  "chat.aiMessage": {
    title: "AI 消息",
    subtitle: "独立为「☀️ 亮色」与「🌙 暗色」声明左侧 AI 回复消息的文字色与气泡底色",
    roles: [
      {
        token: "onSurface",
        partLabel: "文字",
        usageHint: "左侧 AI 气泡与流式回复的正文文字颜色",
      },
      {
        token: "surfaceContainerHighest",
        partLabel: "背景",
        usageHint: "未使用九宫格贴图时的 AI 气泡底色",
      },
      {
        token: "onSurfaceVariant",
        partLabel: "辅助文字",
        usageHint: "代码块标题栏、引用文本、工具调用说明等次要文字颜色",
      },
      {
        token: "tertiaryContainer",
        partLabel: "头像与卡片",
        usageHint: "左侧 AI 机器人头像底色与辅助状态卡片底色",
      },
      {
        token: "onTertiaryContainer",
        partLabel: "图标与卡片文字",
        usageHint: "左侧 AI 头像内部图标与工具卡片文字颜色",
      },
    ],
  },
  "chat.composer": {
    title: "输入框",
    subtitle: "独立控制底部输入容器在「☀️ 亮色」与「🌙 暗色」下的提示词、输入文字、胶囊底色与按钮色；表面配色角色同时用于其他组件",
    roles: [
      {
        token: "onSurface",
        partLabel: "文字",
        usageHint: "模型名称颜色；输入文字继承全局文字主题",
      },
      {
        token: "onSurfaceVariant",
        partLabel: "提示与图标",
        usageHint: "「向 Operit 提问」提示词、展开图标、默认设置/附件图标颜色",
      },
      {
        token: "surfaceContainer",
        partLabel: "Agent 背景",
        usageHint: "Agent 胶囊工作台样式下的输入框背景色（未使用贴图时生效）",
      },
      {
        token: "surface",
        partLabel: "经典背景",
        usageHint: "经典通栏样式下的输入框背景色",
      },
      {
        token: "surfaceContainerHighest",
        partLabel: "附件与模型按钮",
        usageHint: "经典模式附件圆形按钮背景色；模型选择芯片使用透明背景",
      },
      {
        token: "primary",
        partLabel: "发送按钮",
        usageHint: "最右侧主动作按钮在未替换自定义 SVG/PNG 时的圆形背景色",
      },
      {
        token: "onPrimary",
        partLabel: "发送图标",
        usageHint: "最右侧主动作按钮内部麦克风/发送箭头颜色",
      },
      {
        token: "error",
        partLabel: "停止按钮",
        usageHint: "模型输出中时最右侧停止按钮的背景色",
      },
    ],
  },
  background: {
    title: "背景颜色",
    subtitle: "独立控制根画布在无背景图或半透明背景图下的「☀️ 亮色」与「🌙 暗色」衬底表面色",
    roles: [
      {
        token: "surface",
        partLabel: "背景",
        usageHint: "对话区与整体窗口的基准底色（不透明度 < 1 时混合透出）",
      },
      {
        token: "surfaceContainerLowest",
        partLabel: "底层背景",
        usageHint: "底层容器与深色衬底表面",
      },
      {
        token: "surfaceDim",
        partLabel: "次级背景",
        usageHint: "次级背景衬底表面色",
      },
    ],
  },
};

const PALETTE_GROUPS: ReadonlyArray<{ title: string; subtitle: string; keys: string[] }> = [
  {
    title: "核心品牌主色 Primary (8)",
    subtitle: "主要交互按钮、激活指示器、高亮容器与 Fixed 恒定色阶",
    keys: [
      "primary",
      "onPrimary",
      "primaryContainer",
      "onPrimaryContainer",
      "primaryFixed",
      "primaryFixedDim",
      "onPrimaryFixed",
      "onPrimaryFixedVariant",
    ],
  },
  {
    title: "辅助次级色阶 Secondary (8)",
    subtitle: "次要操作芯片、筛选标签、侧栏选中态与 Fixed 恒定色阶",
    keys: [
      "secondary",
      "onSecondary",
      "secondaryContainer",
      "onSecondaryContainer",
      "secondaryFixed",
      "secondaryFixedDim",
      "onSecondaryFixed",
      "onSecondaryFixedVariant",
    ],
  },
  {
    title: "第三点缀色阶 Tertiary (8)",
    subtitle: "徽章强调、特殊状态装饰、对比高光与 Fixed 恒定色阶",
    keys: [
      "tertiary",
      "onTertiary",
      "tertiaryContainer",
      "onTertiaryContainer",
      "tertiaryFixed",
      "tertiaryFixedDim",
      "onTertiaryFixed",
      "onTertiaryFixedVariant",
    ],
  },
  {
    title: "画布与表面容器层级 Surface & Containers (13)",
    subtitle: "窗口表面、五级容器海拔 (Lowest → Highest)、顶栏、侧栏与反转表面",
    keys: [
      "surface",
      "onSurface",
      "surfaceDim",
      "surfaceBright",
      "surfaceContainerLowest",
      "surfaceContainerLow",
      "surfaceContainer",
      "surfaceContainerHigh",
      "surfaceContainerHighest",
      "onSurfaceVariant",
      "surfaceTint",
      "inverseSurface",
      "onInverseSurface",
    ],
  },
  {
    title: "描边、遮罩与语义错误 Outline & Error (8)",
    subtitle: "分割线、边框、阴影遮罩与错误告警色",
    keys: ["outline", "outlineVariant", "shadow", "scrim", "error", "onError", "errorContainer", "onErrorContainer"],
  },
];

const PALETTE_TOKEN_LABELS: Readonly<Record<string, string>> = {
  primary: "主色 Primary",
  onPrimary: "主色上文字 OnPrimary",
  primaryContainer: "主色容器 PrimaryContainer",
  onPrimaryContainer: "主色容器文字 OnPrimaryContainer",
  primaryFixed: "恒定主色 PrimaryFixed",
  primaryFixedDim: "恒定暗主色 PrimaryFixedDim",
  onPrimaryFixed: "恒定主色文字 OnPrimaryFixed",
  onPrimaryFixedVariant: "恒定主色次文字 OnPrimaryFixedVariant",
  secondary: "次色 Secondary",
  onSecondary: "次色上文字 OnSecondary",
  secondaryContainer: "次色容器 SecondaryContainer",
  onSecondaryContainer: "次色容器文字 OnSecondaryContainer",
  secondaryFixed: "恒定次色 SecondaryFixed",
  secondaryFixedDim: "恒定暗次色 SecondaryFixedDim",
  onSecondaryFixed: "恒定次色文字 OnSecondaryFixed",
  onSecondaryFixedVariant: "恒定次色次文字 OnSecondaryFixedVariant",
  tertiary: "第三色 Tertiary",
  onTertiary: "第三色文字 OnTertiary",
  tertiaryContainer: "第三色容器 TertiaryContainer",
  onTertiaryContainer: "第三色容器文字 OnTertiaryContainer",
  tertiaryFixed: "恒定第三色 TertiaryFixed",
  tertiaryFixedDim: "恒定暗第三色 TertiaryFixedDim",
  onTertiaryFixed: "恒定第三色文字 OnTertiaryFixed",
  onTertiaryFixedVariant: "恒定第三色次文字 OnTertiaryFixedVariant",
  error: "错误色 Error",
  onError: "错误色文字 OnError",
  errorContainer: "错误容器 ErrorContainer",
  onErrorContainer: "错误容器文字 OnErrorContainer",
  surface: "主表面 Surface",
  onSurface: "主表面文字 OnSurface",
  surfaceDim: "暗调表面 SurfaceDim",
  surfaceBright: "亮调表面 SurfaceBright",
  surfaceContainerLowest: "最低海拔容器 ContainerLowest",
  surfaceContainerLow: "低海拔容器 ContainerLow",
  surfaceContainer: "标准海拔容器 Container",
  surfaceContainerHigh: "高海拔容器 ContainerHigh",
  surfaceContainerHighest: "最高海拔容器 ContainerHighest",
  onSurfaceVariant: "次级表面文字 OnSurfaceVariant",
  outline: "主描边 Outline",
  outlineVariant: "弱描边 OutlineVariant",
  shadow: "投影色 Shadow",
  scrim: "模态遮罩 Scrim",
  inverseSurface: "反转表面 InverseSurface",
  onInverseSurface: "反转表面文字 OnInverseSurface",
  inversePrimary: "反转主色 InversePrimary",
  surfaceTint: "表面叠色 SurfaceTint",
};

export function defaultTheme(): JsonObject {
  const theme = structuredClone(fixture) as JsonObject;
  const chat = theme.chat as JsonObject;
  chat.style = "bubble";
  return theme;
}

function resolved(node: SchemaNode): SchemaNode {
  if (node.$ref === undefined) return node;
  const name = node.$ref.slice("#/$defs/".length);
  if (node.$ref !== `#/$defs/${name}` || !Object.hasOwn(schema.$defs, name)) {
    throw new Error(`Theme editor cannot resolve ${node.$ref}`);
  }
  return schema.$defs[name];
}

function objectValue(root: JsonObject, path: string[]): unknown {
  let value: unknown = root;
  for (const part of path) {
    if (value === null || typeof value !== "object" || Array.isArray(value)) return undefined;
    value = (value as JsonObject)[part];
  }
  return value;
}

function setValue(root: JsonObject, path: string[], value: unknown): void {
  let parent = root;
  for (const part of path.slice(0, -1)) {
    const next = parent[part];
    if (next === null || typeof next !== "object" || Array.isArray(next)) {
      parent[part] = {};
    }
    parent = parent[part] as JsonObject;
  }
  parent[path.at(-1)!] = value;
}

function expandedDefault(node: SchemaNode, defaultImageAsset: string | null = null): unknown {
  const current = resolved(node);
  if (current.properties === undefined) return structuredClone(current.default ?? null);
  return Object.fromEntries(
    Object.entries(current.properties).map(([key, child]) => {
      if (key === "asset" && child.type === "string") {
        return [key, defaultImageAsset ?? ""];
      }
      return [key, expandedDefault(child, defaultImageAsset)];
    }),
  );
}

/** Ensures UI-expanded bubbleImage without a selected asset serializes as null
 * in canonical theme.json so Rust schema validation never fails on asset: null/"" */
function sanitizeImageContainer(container: JsonObject | undefined): void {
  const img = container?.image as JsonObject | null | undefined;
  if (img != null && typeof img === "object") {
    if (typeof img.asset !== "string" || !img.asset.startsWith("assets/images/")) {
      container!.image = null;
    } else {
      const slice = img.slice as Record<string, number> | undefined;
      if (slice != null && typeof slice === "object") {
        if (typeof slice.xStart === "number" && typeof slice.xEnd === "number" && slice.xEnd < slice.xStart + 0.01) {
          slice.xEnd = Number(Math.min(0.95, slice.xStart + 0.01).toFixed(2));
        }
        if (typeof slice.yStart === "number" && typeof slice.yEnd === "number" && slice.yEnd < slice.yStart + 0.01) {
          slice.yEnd = Number(Math.min(0.95, slice.yStart + 0.01).toFixed(2));
        }
      }
    }
  }
}

function sanitizeThemeForContract(theme: JsonObject): JsonObject {
  const clone = structuredClone(theme);
  const palette = (clone.palette ?? {}) as JsonObject;
  const lightPalette = (palette.light ?? {}) as JsonObject;
  const darkPalette = (palette.dark ?? {}) as JsonObject;

  const migrateSingleColorToDual = (container: JsonObject | undefined, key: string, lightToken: string) => {
    if (!container || typeof container !== "object") return;
    const raw = container[key];
    if (typeof raw === "string" && /^#[0-9A-Fa-f]{8}$/.test(raw)) {
      if (!raw.toUpperCase().endsWith("00")) {
        lightPalette[lightToken] = raw.toUpperCase();
      }
      container[key] = null;
    }
  };

  const chat = clone.chat as JsonObject | undefined;
  if (chat != null && typeof chat === "object") {
    migrateSingleColorToDual(chat, "cursorUserColor", "primaryContainer");
    const userObj = chat.user as JsonObject | undefined;
    migrateSingleColorToDual(userObj, "textColor", "onPrimaryContainer");
    migrateSingleColorToDual(userObj, "color", "primaryContainer");
    const aiObj = chat.ai as JsonObject | undefined;
    migrateSingleColorToDual(aiObj, "textColor", "onSurface");
    migrateSingleColorToDual(aiObj, "color", "surfaceContainerHighest");
    for (const role of ["user", "ai"]) {
      sanitizeImageContainer(chat[role] as JsonObject | undefined);
    }
  }
  const input = clone.input as JsonObject | undefined;
  migrateSingleColorToDual(input, "color", "surfaceContainer");
  sanitizeImageContainer(input);
  if (input != null && typeof input === "object") {
    delete input.actionAsset;
    const allowedInputKeys = new Set(Object.keys(schema.properties?.input?.properties ?? {}));
    for (const k of Object.keys(input)) {
      if (!allowedInputKeys.has(k)) delete input[k];
    }
    for (const key of ["settingsAsset", "attachmentAsset", "micAsset", "sendAsset", "stopAsset"]) {
      const val = input[key];
      if (typeof val !== "string" || !val.startsWith("assets/images/")) {
        input[key] = null;
      }
    }
    const sizeSpecs: Record<string, { def: number; min: number; max: number }> = {
      settingsAssetSize: { def: 18, min: 12, max: 44 },
      attachmentAssetSize: { def: 18, min: 12, max: 44 },
      micAssetSize: { def: 26, min: 12, max: 48 },
      sendAssetSize: { def: 26, min: 12, max: 48 },
      stopAssetSize: { def: 26, min: 12, max: 48 },
    };
    for (const [key, spec] of Object.entries(sizeSpecs)) {
      const raw = input[key];
      if (typeof raw !== "number" || Number.isNaN(raw)) {
        input[key] = spec.def;
      } else {
        input[key] = Math.max(spec.min, Math.min(spec.max, raw));
      }
    }
  }
  return clone;
}

function metaForPath(path: string[]): FieldMeta {
  const joined = path.join(".");
  if (Object.hasOwn(FIELD_META, joined)) return FIELD_META[joined];
  const last = path.at(-1) ?? "";
  if (path[0] === "palette" && Object.hasOwn(PALETTE_TOKEN_LABELS, last)) {
    return { label: PALETTE_TOKEN_LABELS[last] };
  }
  if (joined.endsWith(".image.asset")) {
    return { label: "贴图资源", hint: "支持 PNG / JPEG / 静态 SVG，选择已上传图片或点击右侧直接上传" };
  }
  if (joined.endsWith(".image.renderMode")) {
    return {
      label: "九宫格渲染模式",
      enumLabels: {
        tiled_nine_slice: "平铺九宫格 (Tiled 9-Slice)",
        nine_patch: "拉伸九宫格 (Stretch 9-Patch)",
      },
    };
  }
  if (joined.endsWith(".image.crop")) {
    return { label: "裁切", hint: "裁掉源图四周多余透明边缘的比例" };
  }
  if (joined.endsWith(".image.slice")) {
    return { label: "拉伸区域", hint: "定义水平 X 与垂直 Y 的中心拉伸区比例" };
  }
  if (joined.endsWith(".image.scale")) {
    return { label: "缩放", hint: "控制九宫格四角与边框的渲染缩放倍率 (0.5 ~ 4.0)", unit: "×" };
  }
  const sliceLabels: Record<string, string> = {
    xStart: "水平起点 X-Start",
    xEnd: "水平终点 X-End",
    yStart: "垂直起点 Y-Start",
    yEnd: "垂直终点 Y-End",
  };
  if (Object.hasOwn(sliceLabels, last)) {
    return { label: sliceLabels[last] };
  }
  const boxLabels: Record<string, string> = {
    left: "左 Left",
    top: "上 Top",
    right: "右 Right",
    bottom: "下 Bottom",
  };
  if (Object.hasOwn(boxLabels, last)) {
    const isCropRatio = path.includes("crop");
    return { label: boxLabels[last], unit: isCropRatio ? "" : "px" };
  }
  return { label: last };
}

function rgbaCssFromHex8(hex8: string): string {
  if (!/^#[0-9A-Fa-f]{8}$/.test(hex8)) return "#808080";
  const r = parseInt(hex8.slice(1, 3), 16);
  const g = parseInt(hex8.slice(3, 5), 16);
  const b = parseInt(hex8.slice(5, 7), 16);
  const a = parseInt(hex8.slice(7, 9), 16) / 255;
  return `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
}

/** Schema supplies legal fields, enums, ranges and defaults. Rust remains the
 * only authoring validator; these controls do not confer media authority. */
export class ThemeVisualEditor {
  private theme: JsonObject = defaultTheme();
  private readonly root: HTMLElement;
  private readonly changed: () => void;
  private assets: string[] = [];
  private inspector: ThemeInspectorTarget | null = null;
  private paletteMode: "light" | "dark" = "light";
  private seedColorHex = "#36618E";
  private readonly openState = new Map<string, boolean>();
  private readonly pendingBubbleImagePaths = new Set<string>();
  private readonly requestAssetUpload?: (path: string[]) => void;
  private onClearInspector?: () => void;
  private assetPreviewProvider?: (path: string) => string | null;
  private onRequestBrightness?: (mode: "light" | "dark") => void;
  private onExternalDomSync?: () => void;

  constructor(
    root: HTMLElement,
    changed: () => void,
    requestAssetUpload?: (path: string[]) => void,
    onClearInspector?: () => void,
    assetPreviewProvider?: (path: string) => string | null,
  ) {
    this.assetPreviewProvider = assetPreviewProvider;
    this.root = root;
    this.changed = changed;
    this.requestAssetUpload = requestAssetUpload;
    this.onClearInspector = onClearInspector;
    this.render();
  }

  setClearInspectorHandler(handler: () => void): void {
    this.onClearInspector = handler;
  }

  setAssetPreviewProvider(provider: (path: string) => string | null): void {
    this.assetPreviewProvider = provider;
  }

  setBrightnessRequestHandler(handler: (mode: "light" | "dark") => void): void {
    this.onRequestBrightness = handler;
  }

  setExternalDomSyncHandler(handler: () => void): void {
    this.onExternalDomSync = handler;
  }

  setPaletteMode(mode: "light" | "dark"): void {
    if (this.paletteMode === mode) return;
    this.paletteMode = mode;
    if (this.root.querySelector(".raw-m3-details") != null) {
      this.render();
    }
  }

  get json(): string {
    this.theme = sanitizeThemeForContract(this.theme);
    return JSON.stringify(this.theme, null, 2);
  }

  /**
   * Updates internal theme state from canonical validator output WITHOUT
   * destroying active DOM controls or stealing slider/input focus.
   */
  syncCanonicalJson(source: string): void {
    try {
      const parsed: unknown = JSON.parse(source);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return;
      const nextTheme = sanitizeThemeForContract(parsed as JsonObject);
      for (const pendingKey of this.pendingBubbleImagePaths) {
        const parts = pendingKey.split(".");
        if (objectValue(nextTheme, parts) == null) {
          const currentVal = objectValue(this.theme, parts);
          if (currentVal != null) {
            setValue(nextTheme, parts, structuredClone(currentVal));
          }
        }
      }
      this.theme = nextTheme;
    } catch {
      // Ignore malformed intermediate JSON
    }
  }

  setJson(source: string, notify = true): void {
    const parsed: unknown = JSON.parse(source);
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      throw new Error("theme.json must contain an object");
    }
    const nextTheme = sanitizeThemeForContract(parsed as JsonObject);
    for (const pendingKey of this.pendingBubbleImagePaths) {
      const parts = pendingKey.split(".");
      if (objectValue(nextTheme, parts) == null) {
        const currentVal = objectValue(this.theme, parts);
        if (currentVal != null) {
          setValue(nextTheme, parts, structuredClone(currentVal));
        }
      }
    }
    this.theme = nextTheme;
    this.render();
    if (notify) this.changed();
  }

  setAssets(paths: string[]): void {
    this.assets = [...paths];
    this.render();
  }

  setInspectorTarget(target: ThemeInspectorTarget | null): void {
    closeStudioColorPopover();
    const scrollParent = this.root.closest<HTMLElement>(".editor.panel") ?? this.root.parentElement;
    if (scrollParent) scrollParent.scrollTop = 0;
    this.inspector =
      target == null
        ? null
        : {
            ...target,
            sections: target.sections.map(section => ({
              ...section,
              path: [...section.path],
              paths: section.paths?.map(p => [...p]),
            })),
          };
    this.render();
  }

  resetSection(path: string[]): void {
    const defRoot = defaultTheme();
    const defVal = objectValue(defRoot, path);
    for (const pending of [...this.pendingBubbleImagePaths]) {
      if (pending === path.join(".") || pending.startsWith(path.join(".") + ".")) {
        this.pendingBubbleImagePaths.delete(pending);
      }
    }
    setValue(this.theme, path, structuredClone(defVal));
    this.render();
    this.changed();
  }

  setPathValue(path: string[], value: unknown, notify = true): void {
    if (
      ((path.length === 4 && path[0] === "chat" && path[2] === "image") ||
        (path.length === 3 && path[0] === "input" && path[1] === "image")) &&
      path.at(-1) === "asset"
    ) {
      const bubbleImgPath = path.slice(0, -1);
      if (objectValue(this.theme, bubbleImgPath) == null) {
        const imageNode = this.nodeAt(bubbleImgPath);
        setValue(this.theme, bubbleImgPath, expandedDefault(imageNode, typeof value === "string" ? value : null));
      }
      if (typeof value === "string" && value.startsWith("assets/images/")) {
        this.pendingBubbleImagePaths.delete(bubbleImgPath.join("."));
      }
    }
    setValue(this.theme, path, value);
    this.render();
    if (notify) this.changed();
  }

  private mutate(path: string[], value: unknown, rerender = false): void {
    setValue(this.theme, path, value);
    if (
      ((path.length === 5 && path[0] === "chat" && path[2] === "image" && path[3] === "slice") ||
        (path.length === 4 && path[0] === "input" && path[1] === "image" && path[2] === "slice")) &&
      typeof value === "number"
    ) {
      const sliceObj = objectValue(this.theme, path.slice(0, -1)) as Record<string, number> | undefined;
      const field = path.at(-1)!;
      if (sliceObj != null && typeof sliceObj === "object") {
        if (field === "xStart" && typeof sliceObj.xEnd === "number" && sliceObj.xEnd < value + 0.01) {
          sliceObj.xEnd = Number(Math.min(0.95, value + 0.01).toFixed(2));
          rerender = true;
        } else if (field === "xEnd" && typeof sliceObj.xStart === "number" && sliceObj.xStart > value - 0.01) {
          sliceObj.xStart = Number(Math.max(0.05, value - 0.01).toFixed(2));
          rerender = true;
        } else if (field === "yStart" && typeof sliceObj.yEnd === "number" && sliceObj.yEnd < value + 0.01) {
          sliceObj.yEnd = Number(Math.min(0.95, value + 0.01).toFixed(2));
          rerender = true;
        } else if (field === "yEnd" && typeof sliceObj.yStart === "number" && sliceObj.yStart > value - 0.01) {
          sliceObj.yStart = Number(Math.max(0.05, value - 0.01).toFixed(2));
          rerender = true;
        }
      }
    }
    if (rerender) this.render();
    this.changed();
  }

  private createResetButton(path: string[], label = "重置"): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "section-reset-btn";
    btn.textContent = label;
    btn.title = `将 ${path.join(".")} 恢复为默认主题参数`;
    btn.onclick = event => {
      event.preventDefault();
      event.stopPropagation();
      this.resetSection(path);
    };
    return btn;
  }

  /** Render the selected section without repeating technical metadata. */
  private render(): void {
    const scrollParent = this.root.closest<HTMLElement>(".editor.panel") ?? this.root.parentElement;
    const savedScrollTop = scrollParent?.scrollTop ?? 0;
    this.root.replaceChildren();

    if (this.inspector != null) {
      const heading = document.createElement("div");
      heading.className = "inspector-target-heading";

      const topRow = document.createElement("div");
      topRow.className = "inspector-target-top";

      const badge = document.createElement("span");
      badge.className = "inspector-target-badge";
      badge.textContent = this.inspector.badge ?? "ACTIVE ELEMENT";

      const clearBtn = document.createElement("button");
      clearBtn.type = "button";
      clearBtn.className = "inspector-back-btn";
      clearBtn.textContent = "全部";
      clearBtn.onclick = () => {
        if (this.onClearInspector) {
          this.onClearInspector();
        } else {
          this.setInspectorTarget(null);
        }
      };

      topRow.append(badge, clearBtn);

      const title = document.createElement("h3");
      title.textContent = this.inspector.title;
      const description = document.createElement("p");
      description.textContent = this.inspector.description;
      heading.append(topRow, title);
      heading.title = this.inspector.description;
      this.root.append(heading);

      if (this.inspector.id === "palette") {
        heading.append(this.createResetButton(["palette"]));
        this.root.append(this.renderPaletteStudio());
        if (scrollParent) scrollParent.scrollTop = savedScrollTop;
        this.onExternalDomSync?.();
        return;
      }

      for (const section of this.inspector.sections) {
        const stateKey = `inspector:${this.inspector.id ?? ""}:${section.path.join(".")}`;
        const details = document.createElement("details");
        details.className = "editor-group inspector-target-section";
        details.open = this.openState.get(stateKey) ?? (section.open ?? true);
        details.ontoggle = () => this.openState.set(stateKey, details.open);

        const summary = document.createElement("summary");
        const summaryTitle = document.createElement("span");
        summaryTitle.className = "summary-title";
        summaryTitle.textContent = section.label;

        const summaryRight = document.createElement("div");
        summaryRight.className = "summary-right-cluster";
        const summaryPath = document.createElement("code");
        summaryPath.className = "summary-path";
        summaryPath.textContent = section.path.join(".");
        summaryRight.append(this.createResetButton(section.path));
        summary.append(summaryTitle, summaryRight);

        if (section.paths != null && section.paths.length > 0) {
          const multiWrap = document.createElement("div");
          multiWrap.className = "nested-children";
          for (const subPath of section.paths) {
            multiWrap.append(this.renderSectionBody(this.nodeAt(subPath), subPath));
          }
          details.append(summary, multiWrap);
        } else {
          const node = this.nodeAt(section.path);
          details.append(summary, this.renderSectionBody(node, section.path));
        }
        this.root.append(details);
      }
      if (this.inspector.id && Object.hasOwn(COMPONENT_COLOR_MAP, this.inspector.id)) {
        const dualCard = this.renderComponentDualModeCard(this.inspector.id, false);
        const firstSection = this.root.querySelector(".inspector-target-section");
        if (firstSection) {
          this.root.insertBefore(dualCard, firstSection);
        } else {
          this.root.append(dualCard);
        }
      }
      if (scrollParent) scrollParent.scrollTop = savedScrollTop;
      this.onExternalDomSync?.();
      return;
    }

    const groups: Array<{ title: string; subtitle: string; tag: string; path: string[]; open: boolean }> = [
      {
        title: "背景",
        subtitle: "画布底图、填充方式、不透明度与高斯模糊",
        tag: "background",
        path: ["background"],
        open: true,
      },
      {
        title: "消息",
        subtitle: "气泡/Cursor 范式、头像几何、左右气泡九宫格贴图",
        tag: "chat",
        path: ["chat"],
        open: true,
      },
      {
        title: "输入框",
        subtitle: "输入框造型、九宫格贴图与右侧 5 状态动作图标",
        tag: "input",
        path: ["input"],
        open: true,
      },
      {
        title: "透明表面",
        subtitle: "顶栏、导航侧栏与输入框的全局毛玻璃穿透联动",
        tag: "surfaces",
        path: ["surfaces"],
        open: true,
      },
      {
        title: "字体",
        subtitle: "全局字体与用户/AI 消息专属 TTF 字体映射",
        tag: "typography",
        path: ["typography"],
        open: true,
      },
      {
        title: "调色板",
        subtitle: "亮/暗各 46 个语义色彩角色，支持从主色或背景图一键生成",
        tag: "palette",
        path: ["palette"],
        open: false,
      },
    ];

    for (const group of groups) {
      const stateKey = `group:${group.tag}`;
      const details = document.createElement("details");
      details.className = "editor-group";
      details.dataset.group = group.tag;
      details.open = this.openState.get(stateKey) ?? group.open;
      details.ontoggle = () => this.openState.set(stateKey, details.open);

      const summary = document.createElement("summary");
      const summaryLeft = document.createElement("div");
      summaryLeft.className = "summary-heading";
      const summaryTitle = document.createElement("span");
      summaryTitle.className = "summary-title";
      summaryTitle.textContent = group.title;
      const summaryNote = document.createElement("span");
      summaryNote.className = "summary-subtitle";
      summaryNote.textContent = group.subtitle;
      summaryLeft.append(summaryTitle);

      const summaryRight = document.createElement("div");
      summaryRight.className = "summary-right-cluster";
      const summaryCode = document.createElement("code");
      summaryCode.className = "summary-path";
      summaryCode.textContent = group.tag;
      summaryRight.append(this.createResetButton(group.path));
      summary.append(summaryLeft, summaryRight);

      const body = this.renderSectionBody(resolved(schema.properties![group.path[0]]), group.path);
      details.append(summary, body);
      this.root.append(details);
    }
    if (scrollParent) scrollParent.scrollTop = savedScrollTop;
    this.onExternalDomSync?.();
  }

  private renderSectionBody(node: SchemaNode, path: string[]): HTMLElement {
    if (path.length === 1 && path[0] === "palette") {
      return this.renderPaletteStudio();
    }
    const body = this.renderNode(node, path);
    if (this.inspector == null && path.length === 1) {
      if (path[0] === "background") {
        body.prepend(this.renderComponentDualModeCard("background", false));
      } else if (path[0] === "input") {
        body.prepend(this.renderComponentDualModeCard("chat.composer", false));
      } else if (path[0] === "chat") {
        body.prepend(
          this.renderComponentDualModeCard("chat.userMessage", false),
          this.renderComponentDualModeCard("chat.aiMessage", false),
        );
      }
    }
    return body;
  }



  public renderStageQuickColorBar(targetId: string, onCloseHud?: () => void): HTMLElement {
    const spec = COMPONENT_COLOR_MAP[targetId];
    const bar = document.createElement("div");
    bar.className = "stage-quick-color-bar";
    if (!spec) return bar;

    const topRow = document.createElement("div");
    topRow.className = "stage-quick-top";

    const navSlot = document.createElement("div");
    navSlot.className = "stage-quick-nav-slot";
    topRow.append(navSlot);

    const schemeBar = document.createElement("div");
    schemeBar.className = "stage-inline-presets";
    for (const preset of COMPONENT_DUAL_PRESETS) {
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = "dual-preset-chip";
      chip.textContent = preset.label.replace(" / ", "/");
      chip.title = `一键应用部件明暗预设：${preset.label}`;
      chip.onclick = () => {
        for (const role of spec.roles) {
          const pair = preset.applyRole(role.token);
          setValue(this.theme, ["palette", "light", role.token], pair.light);
          setValue(this.theme, ["palette", "dark", role.token], pair.dark);
        }
        this.render();
        this.changed();
      };
      schemeBar.append(chip);
    }
    topRow.append(schemeBar);

    const activeOverrides = (spec.overrides ?? []).filter(ov => objectValue(this.theme, ov.path) != null);
    if (activeOverrides.length > 0) {
      const unlockBtn = document.createElement("button");
      unlockBtn.type = "button";
      unlockBtn.className = "unlock-dual-color-btn compact-unlock-btn";
      unlockBtn.textContent = "🔓 解除单色锁定以启用明暗双套配色";
      unlockBtn.title = `当前启用了 ${activeOverrides.map(o => o.label).join("、")}，点击切换为亮/暗双套独立色板`;
      unlockBtn.onclick = () => {
        for (const ov of activeOverrides) {
          const curVal = objectValue(this.theme, ov.path);
          if (typeof curVal === "string" && /^#[0-9A-Fa-f]{8}$/.test(curVal) && !curVal.endsWith("00")) {
            setValue(this.theme, ["palette", "light", ov.fallbackLightToken], curVal.toUpperCase());
          }
          setValue(this.theme, ov.path, null);
        }
        this.render();
        this.changed();
      };
      topRow.append(unlockBtn);
    }

    const actionsRight = document.createElement("div");
    actionsRight.className = "dual-color-header-actions";
    const resetBtn = document.createElement("button");
    resetBtn.type = "button";
    resetBtn.className = "section-reset-btn";
    resetBtn.textContent = "重置";
    resetBtn.onclick = () => {
      const def = defaultTheme();
      for (const role of spec.roles) {
        const defLight = objectValue(def, ["palette", "light", role.token]);
        const defDark = objectValue(def, ["palette", "dark", role.token]);
        if (defLight !== undefined) setValue(this.theme, ["palette", "light", role.token], defLight);
        if (defDark !== undefined) setValue(this.theme, ["palette", "dark", role.token], defDark);
      }
      this.render();
      this.changed();
    };
    actionsRight.append(resetBtn);

    if (onCloseHud) {
      const closeBtn = document.createElement("button");
      closeBtn.type = "button";
      closeBtn.className = "hud-close-btn";
      closeBtn.textContent = "✕";
      closeBtn.title = "收起预览区配色浮窗";
      closeBtn.onclick = onCloseHud;
      actionsRight.append(closeBtn);
    }
    topRow.append(actionsRight);

    const chipsRow = document.createElement("div");
    chipsRow.className = "stage-quick-chips";

    for (const role of spec.roles) {
      const pill = document.createElement("div");
      pill.className = "stage-role-pill";
      pill.title = `${role.partLabel} (${role.token})：${role.usageHint}`;

      const label = document.createElement("span");
      label.className = "stage-role-label";
      label.textContent = role.partLabel;

      const pair = document.createElement("div");
      pair.className = "stage-role-swatches";

      const buttonsByMode: Record<"light" | "dark", HTMLButtonElement> = {} as Record<"light" | "dark", HTMLButtonElement>;
      for (const mode of ["light", "dark"] as const) {
        const path = ["palette", mode, role.token];
        const rawVal = objectValue(this.theme, path);
        const validHex =
          typeof rawVal === "string" && /^#[0-9A-Fa-f]{8}$/.test(rawVal)
            ? rawVal.toUpperCase()
            : "#808080FF";

        const swatchBtn = document.createElement("button");
        swatchBtn.type = "button";
        swatchBtn.className = `stage-swatch-btn ${mode}`;
        swatchBtn.title = `点击打开取色面板 · ${role.partLabel} (${mode === "light" ? "☀️ 亮色" : "🌙 暗色"})`;

        const modeIcon = document.createElement("span");
        modeIcon.className = "stage-swatch-mode";
        modeIcon.textContent = mode === "light" ? "☀️" : "🌙";

        const dot = document.createElement("span");
        dot.className = "stage-swatch-dot";
        dot.style.background = rgbaCssFromHex8(validHex);

        swatchBtn.dataset.dualToken = role.token;
        swatchBtn.dataset.dualMode = mode;
        swatchBtn.append(modeIcon, dot);
        buttonsByMode[mode] = swatchBtn;

        swatchBtn.onclick = () => {
          openStudioColorPopover({
            anchor: swatchBtn,
            title: role.partLabel,
            subtitle: role.token,
            initialMode: mode,
            dualMode: true,
            getHex: m => {
              const v = objectValue(this.theme, ["palette", m, role.token]);
              return typeof v === "string" && /^#[0-9A-Fa-f]{8}$/.test(v) ? v.toUpperCase() : "#808080FF";
            },
            onChange: (m, nextHex8) => {
              this.onRequestBrightness?.(m);
              this.mutate(["palette", m, role.token], nextHex8);
              this.syncDualColorDom(m, role.token, nextHex8);
            },
            onReset: m => {
              const defVal = objectValue(defaultTheme(), ["palette", m, role.token]);
              if (typeof defVal === "string") {
                this.onRequestBrightness?.(m);
                this.mutate(["palette", m, role.token], defVal);
                this.syncDualColorDom(m, role.token, defVal);
                return defVal;
              }
              return undefined;
            },
          });
        };

        pair.append(swatchBtn);
      }
      pill.append(label, pair);
      chipsRow.append(pill);
    }

    bar.append(topRow, chipsRow);
    return bar;
  }

  /** Render paired color controls with optional presets. */
  public renderComponentDualModeCard(targetId: string, isStageHud = false, onCloseHud?: () => void): HTMLElement {
    const spec = COMPONENT_COLOR_MAP[targetId];
    const card = document.createElement("div");
    card.className = isStageHud
      ? "dual-mode-color-card stage-hud-color-card"
      : "dual-mode-color-card inspector-dual-color-card";

    if (!spec) return card;

    const header = document.createElement("div");
    header.className = "dual-color-card-header";

    const titleBlock = document.createElement("div");
    titleBlock.className = "dual-color-title-block";
    const h4 = document.createElement("h4");
    h4.textContent = spec.title;
    const sub = document.createElement("p");
    sub.textContent = spec.subtitle;
    titleBlock.append(h4);
    titleBlock.title = spec.subtitle;

    const headerRight = document.createElement("div");
    headerRight.className = "dual-color-header-actions";

    const resetBtn = document.createElement("button");
    resetBtn.type = "button";
    resetBtn.className = "section-reset-btn";
    resetBtn.textContent = "重置";
    resetBtn.onclick = () => {
      const def = defaultTheme();
      for (const role of spec.roles) {
        const defLight = objectValue(def, ["palette", "light", role.token]);
        const defDark = objectValue(def, ["palette", "dark", role.token]);
        if (defLight !== undefined) setValue(this.theme, ["palette", "light", role.token], defLight);
        if (defDark !== undefined) setValue(this.theme, ["palette", "dark", role.token], defDark);
      }
      this.render();
      this.changed();
    };
    headerRight.append(resetBtn);

    if (isStageHud && onCloseHud) {
      const closeBtn = document.createElement("button");
      closeBtn.type = "button";
      closeBtn.className = "hud-close-btn";
      closeBtn.textContent = "✕";
      closeBtn.title = "收起预览区配色浮窗";
      closeBtn.onclick = onCloseHud;
      headerRight.append(closeBtn);
    }

    header.append(titleBlock, headerRight);
    card.append(header);

    const schemeBar = document.createElement("div");
    schemeBar.className = "dual-component-presets-bar";
    const schemeHint = document.createElement("span");
    schemeHint.className = "dual-presets-label";
    schemeHint.textContent = "快速预设：";
    schemeBar.append(schemeHint);
    for (const preset of COMPONENT_DUAL_PRESETS) {
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = "dual-preset-chip";
      chip.textContent = preset.label;
      chip.onclick = () => {
        for (const role of spec.roles) {
          const pair = preset.applyRole(role.token);
          setValue(this.theme, ["palette", "light", role.token], pair.light);
          setValue(this.theme, ["palette", "dark", role.token], pair.dark);
        }
        this.render();
        this.changed();
      };
      schemeBar.append(chip);
    }
    const presets = document.createElement("details");
    presets.className = "color-presets";
    const presetsSummary = document.createElement("summary");
    presetsSummary.textContent = "预设";
    presets.append(presetsSummary, schemeBar);
    card.append(presets);

    // Check if any single-mode fixed color override is currently active on this component
    const activeOverrides = (spec.overrides ?? []).filter(ov => objectValue(this.theme, ov.path) != null);
    if (activeOverrides.length > 0) {
      const banner = document.createElement("div");
      banner.className = "dual-color-override-banner";
      const msg = document.createElement("div");
      msg.className = "dual-color-override-text";
      msg.textContent = "当前使用固定颜色";
      const unlockBtn = document.createElement("button");
      unlockBtn.type = "button";
      unlockBtn.className = "unlock-dual-color-btn";
      unlockBtn.textContent = "分开设置";
      unlockBtn.onclick = () => {
        for (const ov of activeOverrides) {
          const curVal = objectValue(this.theme, ov.path);
          if (typeof curVal === "string" && /^#[0-9A-Fa-f]{8}$/.test(curVal) && !curVal.endsWith("00")) {
            setValue(this.theme, ["palette", "light", ov.fallbackLightToken], curVal.toUpperCase());
          }
          setValue(this.theme, ov.path, null);
        }
        this.render();
        this.changed();
      };
      banner.append(msg, unlockBtn);
      card.append(banner);
    }

    const colHeader = document.createElement("div");
    colHeader.className = "dual-color-table-head";
    colHeader.innerHTML = `
      <span class="dual-col-role">颜色</span>
      <span class="dual-col-mode light-col-head">浅色</span>
      <span class="dual-col-mode dark-col-head">深色</span>`;
    card.append(colHeader);

    const list = document.createElement("div");
    list.className = "dual-color-role-list";
    for (const role of spec.roles) {
      list.append(this.renderDualModeTokenRow(role));
    }
    card.append(list);
    return card;
  }

  /** Render one color role with accessible light and dark controls. */
  private renderDualModeTokenRow(role: ComponentColorRoleSpec): HTMLElement {
    const row = document.createElement("div");
    row.className = "dual-color-row";

    const info = document.createElement("div");
    info.className = "dual-color-info";
    const titleLine = document.createElement("div");
    titleLine.className = "dual-color-title-line";
    const labelSpan = document.createElement("span");
    labelSpan.className = "dual-color-part-label";
    labelSpan.textContent = role.partLabel;
    const tokenCode = document.createElement("code");
    tokenCode.className = "field-key";
    tokenCode.textContent = role.token;
    titleLine.append(labelSpan);
    labelSpan.title = `${role.token}: ${role.usageHint}`;

    const hintEl = document.createElement("small");
    hintEl.className = "dual-color-usage-hint";
    hintEl.textContent = role.usageHint;
    info.append(titleLine);

    const lightTrigger = this.buildDualModeTriggerPill("light", role);
    const darkTrigger = this.buildDualModeTriggerPill("dark", role);

    const openPopoverForMode = (initialMode: "light" | "dark", anchorEl: HTMLElement) => {
      openStudioColorPopover({
        anchor: anchorEl,
        title: role.partLabel,
        subtitle: role.token,
        initialMode,
        dualMode: true,
        getHex: m => {
          const v = objectValue(this.theme, ["palette", m, role.token]);
          return typeof v === "string" && /^#[0-9A-Fa-f]{8}$/.test(v) ? v.toUpperCase() : "#808080FF";
        },
        onChange: (m, nextHex8) => {
          this.onRequestBrightness?.(m);
          this.mutate(["palette", m, role.token], nextHex8);
          this.syncDualColorDom(m, role.token, nextHex8);
        },
        onReset: m => {
          const defVal = objectValue(defaultTheme(), ["palette", m, role.token]);
          if (typeof defVal === "string") {
            this.onRequestBrightness?.(m);
            this.mutate(["palette", m, role.token], defVal);
            this.syncDualColorDom(m, role.token, defVal);
            return defVal;
          }
          return undefined;
        },
      });
    };

    lightTrigger.onclick = () => openPopoverForMode("light", lightTrigger);
    darkTrigger.onclick = () => openPopoverForMode("dark", darkTrigger);

    row.append(info, lightTrigger, darkTrigger);
    return row;
  }


  private syncDualColorDom(mode: "light" | "dark", token: string, hex8: string): void {
    if (typeof document === "undefined") return;
    const selector = `[data-dual-token="${token}"][data-dual-mode="${mode}"]`;
    for (const el of Array.from(document.querySelectorAll<HTMLElement>(selector))) {
      const swatch = el.querySelector<HTMLElement>(".dual-trigger-swatch");
      const code = el.querySelector<HTMLElement>(".dual-trigger-hex");
      const alpha = el.querySelector<HTMLElement>(".dual-trigger-alpha");
      const stageDot = el.querySelector<HTMLElement>(".stage-swatch-dot");
      if (swatch) swatch.style.setProperty("--swatch-color", rgbaCssFromHex8(hex8));
      if (code) code.textContent = hex8.slice(0, 7);
      if (alpha) {
        const pct = Math.round((parseInt(hex8.slice(7, 9), 16) / 255) * 100);
        alpha.textContent = pct < 100 ? `${pct}%` : "";
        alpha.classList.toggle("is-translucent", pct < 100);
      }
      if (stageDot) stageDot.style.background = rgbaCssFromHex8(hex8);
    }
  }

  private buildDualModeTriggerPill(mode: "light" | "dark", role: ComponentColorRoleSpec): HTMLButtonElement {
    const rawVal = objectValue(this.theme, ["palette", mode, role.token]);
    const validHex =
      typeof rawVal === "string" && /^#[0-9A-Fa-f]{8}$/.test(rawVal)
        ? rawVal.toUpperCase()
        : "#808080FF";
    const alphaPct = Math.round((parseInt(validHex.slice(7, 9), 16) / 255) * 100);

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `dual-color-trigger dual-trigger-${mode}`;
    btn.dataset.dualToken = role.token;
    btn.dataset.dualMode = mode;
    btn.title = `${role.partLabel} · ${mode === "light" ? "浅色" : "深色"}`;
    btn.setAttribute("aria-label", btn.title);

    const modeIcon = document.createElement("span");
    modeIcon.className = "dual-trigger-mode";
    modeIcon.textContent = mode === "light" ? "☀️" : "🌙";

    const swatch = document.createElement("span");
    swatch.className = "dual-trigger-swatch";
    swatch.style.setProperty("--swatch-color", rgbaCssFromHex8(validHex));

    const hexText = document.createElement("code");
    hexText.className = "dual-trigger-hex";
    hexText.textContent = validHex.slice(0, 7);

    const alphaText = document.createElement("span");
    alphaText.className = `dual-trigger-alpha${alphaPct < 100 ? " is-translucent" : ""}`;
    alphaText.textContent = alphaPct < 100 ? `${alphaPct}%` : "";

    btn.append(modeIcon, swatch, hexText, alphaText);
    return btn;
  }

  /** Render seed controls and component colors without redundant section headings. */
  private renderPaletteStudio(): HTMLElement {
    const wrapper = document.createElement("div");
    wrapper.className = "palette-studio";

    const genCard = document.createElement("div");
    genCard.className = "palette-generator-card";

    const genControls = document.createElement("div");
    genControls.className = "palette-generator-controls";

    const seedPickerWrap = document.createElement("label");
    seedPickerWrap.className = "seed-picker-pill";
    const seedColorInput = document.createElement("input");
    seedColorInput.type = "color";
    seedColorInput.setAttribute("aria-label", "主色");
    seedColorInput.value = this.seedColorHex;
    const seedHexLabel = document.createElement("code");
    seedHexLabel.className = "seed-hex-label";
    seedHexLabel.textContent = this.seedColorHex;
    seedColorInput.oninput = () => {
      this.seedColorHex = seedColorInput.value.toUpperCase();
      seedHexLabel.textContent = this.seedColorHex;
    };
    seedPickerWrap.append(seedColorInput, seedHexLabel);

    const applySeedBtn = document.createElement("button");
    applySeedBtn.type = "button";
    applySeedBtn.className = "secondary-compact-btn accent-btn";
    applySeedBtn.textContent = "应用主色";
    applySeedBtn.title = "用主色重新生成浅色与深色配色";
    applySeedBtn.onclick = () => {
      const generated = generateDualModePaletteFromSeed(this.seedColorHex);
      setValue(this.theme, ["palette", "light"], generated.light);
      setValue(this.theme, ["palette", "dark"], generated.dark);
      this.render();
      this.changed();
    };

    const bgAssetPath = objectValue(this.theme, ["background", "asset"]);
    const bgPreviewUrl =
      typeof bgAssetPath === "string" && bgAssetPath.length > 0
        ? this.assetPreviewProvider?.(bgAssetPath) ?? null
        : null;

    const extractBgBtn = document.createElement("button");
    extractBgBtn.type = "button";
    extractBgBtn.className = "secondary-compact-btn";
    extractBgBtn.textContent = "从图片取色";
    extractBgBtn.disabled = bgPreviewUrl == null;
    extractBgBtn.title =
      bgPreviewUrl == null ? "请先在「背景」上传并绑定背景图" : "从当前背景图提取代表色并生成亮/暗双模色板";
    extractBgBtn.onclick = async () => {
      if (!bgPreviewUrl) return;
      try {
        extractBgBtn.disabled = true;
        extractBgBtn.textContent = "正在取色…";
        const extractedHex = await extractSeedColorFromImageUrl(bgPreviewUrl);
        this.seedColorHex = extractedHex;
        const generated = generateDualModePaletteFromSeed(extractedHex);
        setValue(this.theme, ["palette", "light"], generated.light);
        setValue(this.theme, ["palette", "dark"], generated.dark);
        this.render();
        this.changed();
      } finally {
        extractBgBtn.disabled = false;
        extractBgBtn.textContent = "从图片取色";
      }
    };

    genControls.append(seedPickerWrap, applySeedBtn, extractBgBtn);

    const presetRow = document.createElement("div");
    presetRow.className = "palette-seed-presets";
    for (const preset of PALETTE_SEED_PRESETS) {
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = "seed-preset-chip";
      const dot = document.createElement("span");
      dot.className = "seed-preset-dot";
      dot.style.background = preset.seedHex;
      chip.title = `${preset.label} · ${preset.seedHex}`;
      chip.setAttribute("aria-label", `应用${preset.label}主色`);
      chip.setAttribute("aria-pressed", String(this.seedColorHex.toUpperCase() === preset.seedHex.toUpperCase()));
      dot.setAttribute("aria-hidden", "true");
      chip.append(dot);
      chip.onclick = () => {
        this.seedColorHex = preset.seedHex;
        const generated = generateDualModePaletteFromSeed(preset.seedHex);
        setValue(this.theme, ["palette", "light"], generated.light);
        setValue(this.theme, ["palette", "dark"], generated.dark);
        this.render();
        this.changed();
      };
      presetRow.append(chip);
    }

    genCard.append(genControls, presetRow);
    wrapper.append(genCard);

    const ribbon = document.createElement("div");
    ribbon.className = "palette-ribbon";
    const activePalette = (objectValue(this.theme, ["palette", this.paletteMode]) ?? {}) as Record<string, string>;
    const previewKeys = [
      "primary",
      "primaryContainer",
      "secondary",
      "secondaryContainer",
      "tertiary",
      "surface",
      "surfaceContainerLow",
      "surfaceContainerHigh",
      "error",
    ];
    for (const key of previewKeys) {
      const hex = activePalette[key] ?? "#808080FF";
      const swatch = document.createElement("span");
      swatch.className = "palette-ribbon-chip";
      swatch.title = `${PALETTE_TOKEN_LABELS[key] ?? key}: ${hex}`;
      swatch.style.setProperty("--swatch-color", rgbaCssFromHex8(hex));
      ribbon.append(swatch);
    }

    const modeTabs = document.createElement("div");
    modeTabs.className = "palette-mode-tabs";
    const modes: Array<{ id: "light" | "dark"; label: string }> = [
      { id: "light", label: "浅色" },
      { id: "dark", label: "深色" },
    ];
    for (const item of modes) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = `palette-mode-tab${this.paletteMode === item.id ? " active" : ""}`;
      btn.textContent = item.label;
      btn.onclick = () => this.setPaletteMode(item.id);
      modeTabs.append(btn);
    }

    const headerBar = document.createElement("div");
    headerBar.className = "palette-studio-header";
    headerBar.append(modeTabs, ribbon);
    // 1. Component-oriented Dual-Mode Color Cards (clear UI mapping, Light + Dark side-by-side)
    for (const targetKey of [
      "navigation.sidebar",
      "chat.topBar",
      "chat.userMessage",
      "chat.aiMessage",
      "chat.composer",
      "background",
    ]) {
      wrapper.append(this.renderComponentDualModeCard(targetKey, false));
    }

    // 2. Advanced full 46-token M3 raw table (collapsible)
    const rawDetails = document.createElement("details");
    rawDetails.className = "editor-group raw-m3-details";
    const rawStateKey = "palette:raw-46";
    rawDetails.open = this.openState.get(rawStateKey) ?? false;
    rawDetails.ontoggle = () => this.openState.set(rawStateKey, rawDetails.open);
    const rawSummary = document.createElement("summary");
    rawSummary.innerHTML = `
      <div class="summary-heading">
        <span class="summary-title">全部颜色</span>
        <span class="summary-subtitle">包含 Fixed、Inverse、SurfaceContainer 各海拔与 Scrim/Shadow 全部原始字段</span>
      </div>
      <code class="summary-path">46 × 2</code>`;
    const rawBody = document.createElement("div");
    rawBody.className = "nested-children";
    rawBody.append(headerBar);

    const modeNode = this.nodeAt(["palette", this.paletteMode]);
    for (const group of PALETTE_GROUPS) {
      const groupCard = document.createElement("div");
      groupCard.className = "subcard palette-group-card";
      const groupHeader = document.createElement("div");
      groupHeader.className = "subcard-header";
      const groupTitle = document.createElement("h4");
      groupTitle.textContent = group.title;
      const groupSub = document.createElement("p");
      groupSub.textContent = group.subtitle;
      groupHeader.append(groupTitle, groupSub);
      groupCard.append(groupHeader);

      const grid = document.createElement("div");
      grid.className = "palette-token-grid";
      for (const tokenKey of group.keys) {
        const tokenSchema = modeNode.properties?.[tokenKey];
        if (tokenSchema != null) {
          grid.append(this.renderNode(tokenSchema, ["palette", this.paletteMode, tokenKey]));
        }
      }
      groupCard.append(grid);
      rawBody.append(groupCard);
    }
    rawDetails.append(rawSummary, rawBody);
    wrapper.append(rawDetails);
    return wrapper;
  }

  private renderInputActionSlots(): HTMLElement {
    const card = document.createElement("div");
    card.className = "subcard input-action-slots-card";

    const header = document.createElement("div");
    header.className = "subcard-header";
    const titleRow = document.createElement("div");
    titleRow.className = "subcard-title-row";
    const h4 = document.createElement("h4");
    h4.textContent = "输入栏右侧动作图标与尺寸 (5 状态槽位)";
    const resetAllBtn = document.createElement("button");
    resetAllBtn.type = "button";
    resetAllBtn.className = "section-reset-btn";
    resetAllBtn.textContent = "↺ 恢复默认图标";
    resetAllBtn.onclick = () => {
      for (const slot of INPUT_ACTION_SLOTS) {
        setValue(this.theme, ["input", slot.assetKey], null);
        setValue(this.theme, ["input", slot.sizeKey], slot.defaultSize);
      }
      this.render();
      this.changed();
    };
    titleRow.append(h4, resetAllBtn);

    const hint = document.createElement("p");
    hint.textContent = "支持 PNG / JPEG / 静态无损 SVG。可在右侧预览画布点击最右侧按钮循环切换「语音 → 发送 → 停止」三态查看效果。";
    header.append(titleRow, hint);
    card.append(header);

    const grid = document.createElement("div");
    grid.className = "action-slot-grid";
    const imageAssets = this.assets.filter(a => a.startsWith("assets/images/"));

    for (const slot of INPUT_ACTION_SLOTS) {
      const slotEl = document.createElement("div");
      slotEl.className = "action-slot-item";

      const assetPath = objectValue(this.theme, ["input", slot.assetKey]);
      const currentAsset = typeof assetPath === "string" && assetPath.startsWith("assets/images/") ? assetPath : "";
      const currentSizeRaw = objectValue(this.theme, ["input", slot.sizeKey]);
      const currentSize =
        typeof currentSizeRaw === "number" && !Number.isNaN(currentSizeRaw) ? currentSizeRaw : slot.defaultSize;
      const previewUrl = currentAsset ? this.assetPreviewProvider?.(currentAsset) ?? null : null;

      // Top row: Thumbnail + Title + Upload/Clear
      const slotTop = document.createElement("div");
      slotTop.className = "action-slot-top";

      const thumb = document.createElement("div");
      thumb.className = `action-slot-thumb${previewUrl ? " has-image" : ""}`;
      if (previewUrl) {
        const img = document.createElement("img");
        img.src = previewUrl;
        img.alt = slot.title;
        thumb.append(img);
      } else {
        const glyph = document.createElement("span");
        glyph.className = "action-slot-glyph";
        glyph.textContent = slot.glyph;
        thumb.append(glyph);
      }

      const metaBox = document.createElement("div");
      metaBox.className = "action-slot-meta";
      const titleLine = document.createElement("div");
      titleLine.className = "action-slot-title-line";
      const nameSpan = document.createElement("span");
      nameSpan.className = "action-slot-title";
      nameSpan.textContent = slot.title;
      const codeSpan = document.createElement("code");
      codeSpan.className = "field-key";
      codeSpan.textContent = slot.assetKey;
      titleLine.append(nameSpan, codeSpan);

      const subLine = document.createElement("small");
      subLine.className = "action-slot-sub";
      subLine.textContent = slot.subtitle;
      metaBox.append(titleLine, subLine);

      const btnCluster = document.createElement("div");
      btnCluster.className = "action-slot-btns";
      const uploadBtn = document.createElement("button");
      uploadBtn.type = "button";
      uploadBtn.className = "upload-inline-btn";
      uploadBtn.textContent = "上传";
      uploadBtn.onclick = () => this.requestAssetUpload?.(["input", slot.assetKey]);
      btnCluster.append(uploadBtn);

      if (currentAsset) {
        const clearBtn = document.createElement("button");
        clearBtn.type = "button";
        clearBtn.className = "clear-inline-btn";
        clearBtn.textContent = "✕";
        clearBtn.title = "清除此图标素材，恢复默认矢量图标";
        clearBtn.onclick = () => this.mutate(["input", slot.assetKey], null, true);
        btnCluster.append(clearBtn);
      }

      slotTop.append(thumb, metaBox, btnCluster);

      // Middle row: Select asset path
      const select = document.createElement("select");
      select.className = "action-slot-select";
      const options = ["", ...imageAssets];
      if (currentAsset && !options.includes(currentAsset)) options.push(currentAsset);
      for (const opt of options) {
        const el = document.createElement("option");
        el.value = opt;
        el.textContent = opt || "默认内置图标 (None)";
        select.append(el);
      }
      select.value = currentAsset;
      select.onchange = () => {
        this.mutate(["input", slot.assetKey], select.value === "" ? null : select.value, true);
      };

      // Bottom row: Compact Size Slider + Stepper
      const sizeRow = document.createElement("div");
      sizeRow.className = "action-slot-size-row";
      const sizeLabel = document.createElement("span");
      sizeLabel.className = "action-slot-size-label";
      sizeLabel.textContent = `显示尺寸 (${slot.minSize}~${slot.maxSize}px)`;

      const range = document.createElement("input");
      range.type = "range";
      range.className = "scrubber-range";
      range.min = String(slot.minSize);
      range.max = String(slot.maxSize);
      range.step = "1";
      range.value = String(currentSize);

      const numberWrap = document.createElement("div");
      numberWrap.className = "scrubber-number-wrap";
      const numInput = document.createElement("input");
      numInput.type = "number";
      numInput.min = String(slot.minSize);
      numInput.max = String(slot.maxSize);
      numInput.step = "1";
      numInput.value = String(currentSize);
      const unitSpan = document.createElement("span");
      unitSpan.className = "scrubber-unit";
      unitSpan.textContent = "px";
      numberWrap.append(numInput, unitSpan);

      const clampSize = (v: number) => Math.max(slot.minSize, Math.min(slot.maxSize, Math.round(v)));
      range.oninput = () => {
        const c = clampSize(Number(range.value));
        numInput.value = String(c);
        this.mutate(["input", slot.sizeKey], c);
      };
      numInput.onchange = () => {
        const raw = Number(numInput.value);
        const c = Number.isNaN(raw) ? slot.defaultSize : clampSize(raw);
        numInput.value = String(c);
        range.value = String(c);
        this.mutate(["input", slot.sizeKey], c);
      };

      sizeRow.append(sizeLabel, range, numberWrap);
      slotEl.append(slotTop, select, sizeRow);
      grid.append(slotEl);
    }

    card.append(grid);
    return card;
  }

  private nodeAt(path: string[]): SchemaNode {
    let node = schema as SchemaNode;
    for (const part of path) {
      node = resolved(node);
      const child = node.properties?.[part];
      if (child == null) throw new Error(`Unknown theme schema path: ${path.join(".")}`);
      node = child;
    }
    return resolved(node);
  }

  private renderNode(sourceNode: SchemaNode, path: string[]): HTMLElement {
    const node = resolved(sourceNode);
    const value = objectValue(this.theme, path);
    const nullable = Array.isArray(node.type) && node.type.includes("null");
    const name = path.at(-1)!;
    const meta = metaForPath(path);

    if (node.properties !== undefined) {
      const isBoxGrid =
        name === "padding" ||
        name === "centerSlice" ||
        Object.keys(node.properties).every(k => ["left", "top", "right", "bottom"].includes(k));
      const isElevatedCard =
        (path[0] === "chat" && (name === "user" || name === "ai")) ||
        name === "image" ||
        name === "blur";

      const container = document.createElement("div");
      container.className = isElevatedCard
        ? `nested-fields subcard subcard-${name}`
        : isBoxGrid
          ? "nested-fields box-model-card"
          : "nested-fields";

      if (nullable) {
        const pathKey = path.join(".");
        const headerRow = document.createElement("div");
        headerRow.className = "nullable-object-header";
        const toggle = document.createElement("input");
        toggle.type = "checkbox";
        toggle.className = "switch-input";
        toggle.checked = value !== null || this.pendingBubbleImagePaths.has(pathKey);
        toggle.onchange = () => {
          if (toggle.checked) {
            const firstImage = this.assets.find(asset => asset.startsWith("assets/images/")) ?? null;
            if (firstImage == null) {
              this.pendingBubbleImagePaths.add(pathKey);
            } else {
              this.pendingBubbleImagePaths.delete(pathKey);
            }
            this.mutate(path, expandedDefault(node, firstImage), true);
          } else {
            this.pendingBubbleImagePaths.delete(pathKey);
            this.mutate(path, null, true);
          }
        };
        headerRow.append(this.buildFieldLabel(`启用 ${meta.label}`, name, meta.hint, toggle, true));
        container.append(headerRow);
      } else if (path.length > 1) {
        const subHeader = document.createElement("div");
        subHeader.className = "subcard-header";
        const subTitleRow = document.createElement("div");
        subTitleRow.className = "subcard-title-row";
        const subTitle = document.createElement("h4");
        subTitle.textContent = meta.label;
        const rightWrap = document.createElement("div");
        rightWrap.className = "summary-right-cluster";
        const subCode = document.createElement("code");
        subCode.textContent = path.slice(1).join(".");
        if (path[0] === "chat" && (name === "user" || name === "ai")) {
          rightWrap.append(this.createResetButton(path, "重置"), subCode);
        } else {
          rightWrap.append(subCode);
        }
        subTitleRow.append(subTitle, rightWrap);
        subHeader.append(subTitleRow);
        if (meta.hint) subHeader.title = meta.hint;
        container.append(subHeader);
      }

      const isBubbleImage =
        ((path.length === 3 && path[0] === "chat") || (path.length === 2 && path[0] === "input")) && name === "image";
      const isInputRoot = path.length === 1 && path[0] === "input";
      const isExpandedNullable = nullable && (value !== null || this.pendingBubbleImagePaths.has(path.join(".")));

      if (!nullable || isExpandedNullable) {
        if (isBubbleImage && value === null) {
          const firstImage = this.assets.find(asset => asset.startsWith("assets/images/")) ?? null;
          setValue(this.theme, path, expandedDefault(node, firstImage));
        }
        const childrenWrap = document.createElement("div");
        childrenWrap.className = isBoxGrid ? "box-model-grid" : "nested-children";

        if (isBubbleImage) {
          // Render asset, renderMode, scale, 9-slice visual canvas, and a compact collapsible numeric drawer for crop & slice
          for (const key of ["asset", "renderMode", "scale"]) {
            const child = node.properties[key];
            if (child != null) {
              childrenWrap.append(this.renderNode(child, [...path, key]));
            }
          }
          childrenWrap.append(this.renderNineSliceWorkbench(path));

          const numericDetails = document.createElement("details");
          numericDetails.className = "nine-slice-numeric-details";
          const numStateKey = `nineslice-num:${path.join(".")}`;
          numericDetails.open = this.openState.get(numStateKey) ?? false;
          numericDetails.ontoggle = () => this.openState.set(numStateKey, numericDetails.open);
          const numSummary = document.createElement("summary");
          numSummary.innerHTML = `
            <span class="nine-slice-num-title">精确数值微调 (Crop 裁切 & Slice 拉伸区间)</span>
            <code class="field-key">0.00 ~ 0.95</code>`;
          const numBody = document.createElement("div");
          numBody.className = "nested-children";
          for (const key of ["crop", "slice"]) {
            const child = node.properties[key];
            if (child != null) {
              numBody.append(this.renderNode(child, [...path, key]));
            }
          }
          numericDetails.append(numSummary, numBody);
          childrenWrap.append(numericDetails);
        } else if (isInputRoot) {
          for (const [key, child] of Object.entries(node.properties)) {
            if (INPUT_ACTION_KEY_SET.has(key) || LEGACY_SINGLE_COLOR_PATHS.has([...path, key].join("."))) continue;
            childrenWrap.append(this.renderNode(child, [...path, key]));
          }
          childrenWrap.append(this.renderInputActionSlots());
        } else {
          for (const [key, child] of Object.entries(node.properties)) {
            if (LEGACY_SINGLE_COLOR_PATHS.has([...path, key].join("."))) continue;
            childrenWrap.append(this.renderNode(child, [...path, key]));
          }
        }
        container.append(childrenWrap);
      }
      return container;
    }

    if (node.type === "boolean") {
      const input = document.createElement("input");
      input.type = "checkbox";
      input.className = "switch-input";
      input.checked = value === true;
      input.onchange = () => this.mutate(path, input.checked);
      return this.buildFieldLabel(meta.label, name, meta.hint, input, true);
    }

    if (node.enum !== undefined || this.isAssetReference(path) || path[0] === "typography") {
      const select = document.createElement("select");
      const values =
        node.enum ??
        (path[0] === "typography"
          ? ["noto-sans-sc", "cascadia-mono", ...this.assets.filter(asset => asset.startsWith("assets/fonts/"))]
          : this.assets.filter(asset => asset.startsWith("assets/images/")));
      const hasEmptyOption = nullable || (this.isAssetReference(path) && (typeof value !== "string" || value === ""));
      if (hasEmptyOption && !values.includes("")) values.unshift("");
      if (typeof value === "string" && value !== "" && !values.includes(value)) values.push(value);
      for (const entry of values) {
        const option = document.createElement("option");
        option.value = entry;
        option.textContent =
          meta.enumLabels?.[entry] ||
          entry ||
          (path[0] === "typography"
            ? "继承全局字体 (Inherit)"
            : nullable
              ? "不使用资源 (None)"
              : "请选择或上传贴图资源…");
        select.append(option);
      }
      select.value = typeof value === "string" ? value : "";
      select.onchange = () => {
        if (
          ((path.length === 4 && path[0] === "chat" && path[2] === "image") ||
            (path.length === 3 && path[0] === "input" && path[1] === "image")) &&
          path.at(-1) === "asset"
        ) {
          const key = path.slice(0, -1).join(".");
          if (select.value.startsWith("assets/images/")) {
            this.pendingBubbleImagePaths.delete(key);
          } else {
            this.pendingBubbleImagePaths.add(key);
          }
        }
        this.mutate(path, nullable && select.value === "" ? null : select.value, true);
      };

      if (!this.isAssetReference(path)) {
        return this.buildFieldLabel(meta.label, name, meta.hint, select);
      }

      const control = document.createElement("div");
      control.className = "asset-path-control";
      const actionBar = document.createElement("div");
      actionBar.className = "asset-select-row";

      if (path[0] !== "typography") {
        const previewUrl =
          typeof value === "string" && value.startsWith("assets/images/")
            ? this.assetPreviewProvider?.(value) ?? null
            : null;
        const miniThumb = document.createElement("div");
        miniThumb.className = `asset-inline-thumb${previewUrl ? " has-image" : ""}`;
        if (previewUrl && typeof value === "string") {
          const img = document.createElement("img");
          img.src = previewUrl;
          img.alt = value;
          miniThumb.append(img);
        } else {
          miniThumb.textContent = "🖼️";
        }
        actionBar.append(miniThumb);
      }

      const addAsset = document.createElement("button");
      addAsset.type = "button";
      addAsset.className = "upload-inline-btn";
      addAsset.textContent = path[0] === "typography" ? "上传字体" : "上传素材";
      addAsset.onclick = () => this.requestAssetUpload?.([...path]);
      actionBar.append(select, addAsset);

      if (nullable && typeof value === "string" && value !== "") {
        const clearBtn = document.createElement("button");
        clearBtn.type = "button";
        clearBtn.className = "clear-inline-btn";
        clearBtn.textContent = "✕";
        clearBtn.title = "清除绑定的资源";
        clearBtn.onclick = () => this.mutate(path, null, true);
        actionBar.append(clearBtn);
      }

      control.append(this.buildFieldLabel(meta.label, name, meta.hint, actionBar));
      return control;
    }

    if (node.type === "number" || node.type === "integer") {
      const input = document.createElement("input");
      input.type = "number";
      if (node.minimum !== undefined) input.min = String(node.minimum);
      if (node.maximum !== undefined) input.max = String(node.maximum);
      const step = node.type === "integer" || meta.unit === "px" ? "1" : "0.01";
      input.step = step;
      input.value = typeof value === "number" ? String(value) : "";

      if (node.minimum !== undefined && node.maximum !== undefined) {
        const combo = document.createElement("div");
        combo.className = "scrubber-combo";
        const range = document.createElement("input");
        range.type = "range";
        range.className = "scrubber-range";
        range.min = String(node.minimum);
        range.max = String(node.maximum);
        range.step = step;
        range.value = typeof value === "number" ? String(value) : String(node.minimum);

        const clampNumber = (raw: number): number => {
          let n = raw;
          if (node.minimum !== undefined) n = Math.max(node.minimum, n);
          if (node.maximum !== undefined) n = Math.min(node.maximum, n);
          return node.type === "integer" ? Math.round(n) : Number(n.toFixed(4));
        };
        range.oninput = () => {
          const clean = clampNumber(Number(range.value));
          input.value = String(clean);
          this.mutate(path, clean);
        };
        input.oninput = () => {
          if (input.value.trim() === "") return;
          const raw = Number(input.value);
          if (!Number.isNaN(raw)) {
            const clean = clampNumber(raw);
            range.value = String(clean);
            this.mutate(path, clean);
          }
        };
        input.onchange = () => {
          const raw = input.value.trim() === "" ? Number(node.default ?? node.minimum ?? 0) : Number(input.value);
          const clean = Number.isNaN(raw) ? Number(node.default ?? node.minimum ?? 0) : clampNumber(raw);
          input.value = String(clean);
          range.value = String(clean);
          this.mutate(path, clean);
        };

        const numberWrap = document.createElement("div");
        numberWrap.className = "scrubber-number-wrap";
        numberWrap.append(input);
        if (meta.unit) {
          const unitSpan = document.createElement("span");
          unitSpan.className = "scrubber-unit";
          unitSpan.textContent = meta.unit;
          numberWrap.append(unitSpan);
        }
        combo.append(range, numberWrap);
        return this.buildFieldLabel(meta.label, name, meta.hint, combo);
      }

      input.onchange = () => this.mutate(path, input.value === "" ? null : Number(input.value));
      return this.buildFieldLabel(meta.label, name, meta.hint, input);
    }

    if (nullable && (value === null || this.isColor(node))) {
      const wrapper = document.createElement("div");
      wrapper.className = "nullable-field";
      const toggle = document.createElement("input");
      toggle.type = "checkbox";
      toggle.className = "switch-input";
      toggle.checked = value !== null;
      toggle.onchange = () => this.mutate(path, toggle.checked ? (this.isColor(node) ? "#3B82F6FF" : "") : null, true);
      wrapper.append(this.buildFieldLabel(`自定义 ${meta.label}`, name, meta.hint, toggle, true));
      if (!toggle.checked) return wrapper;
      wrapper.append(this.textControl(node, path, String(value ?? "")));
      return wrapper;
    }

    return this.textControl(node, path, typeof value === "string" ? value : "");
  }

  private textControl(node: SchemaNode, path: string[], value: string): HTMLElement {
    const name = path.at(-1)!;
    const meta = metaForPath(path);
    const input = document.createElement("input");
    input.type = "text";
    input.value = value;

    if (this.isColor(node)) {
      input.pattern = "#[0-9A-F]{8}";
      input.className = "hex-color-input";
      const validHex = /^#[0-9A-Fa-f]{8}$/.test(value) ? value.toUpperCase() : "#808080FF";

      const swatchBtn = document.createElement("button");
      swatchBtn.type = "button";
      swatchBtn.className = "color-swatch-well";
      swatchBtn.style.setProperty("--swatch-color", rgbaCssFromHex8(validHex));
      swatchBtn.title = "点击打开取色面板（含黑白灰与主题预设）";

      const alphaByte = parseInt(validHex.slice(7, 9), 16);
      const alphaPercent = Math.round((alphaByte / 255) * 100);
      const alphaSlider = document.createElement("input");
      alphaSlider.type = "range";
      alphaSlider.className = "alpha-slider";
      alphaSlider.min = "0";
      alphaSlider.max = "100";
      alphaSlider.step = "1";
      alphaSlider.value = String(alphaPercent);
      alphaSlider.title = "Alpha 不透明度 (%)";

      const alphaBadge = document.createElement("span");
      alphaBadge.className = "alpha-badge";
      alphaBadge.textContent = `${alphaPercent}%`;

      const syncColorVisuals = (hex8: string) => {
        if (/^#[0-9A-Fa-f]{8}$/.test(hex8)) {
          swatchBtn.style.setProperty("--swatch-color", rgbaCssFromHex8(hex8));
          const a = Math.round((parseInt(hex8.slice(7, 9), 16) / 255) * 100);
          alphaSlider.value = String(a);
          alphaBadge.textContent = `${a}%`;
        }
      };

      swatchBtn.onclick = () => {
        openStudioColorPopover({
          anchor: swatchBtn,
          title: meta.label,
          subtitle: path.join("."),
          dualMode: false,
          getHex: () => (/^#[0-9A-Fa-f]{8}$/.test(input.value) ? input.value.toUpperCase() : validHex),
          onChange: (_, nextHex8) => {
            input.value = nextHex8;
            syncColorVisuals(nextHex8);
            this.mutate(path, nextHex8);
          },
          onReset: () => {
            const defVal = objectValue(defaultTheme(), path);
            const fallback = typeof defVal === "string" ? defVal : "#3B82F6FF";
            input.value = fallback;
            syncColorVisuals(fallback);
            this.mutate(path, fallback);
            return fallback;
          },
        });
      };

      alphaSlider.oninput = () => {
        const pct = Number(alphaSlider.value);
        const alphaHex = Math.round((pct / 100) * 255)
          .toString(16)
          .toUpperCase()
          .padStart(2, "0");
        const rgbPart = /^#[0-9A-Fa-f]{6}/.test(input.value)
          ? input.value.slice(0, 7).toUpperCase()
          : validHex.slice(0, 7);
        const nextHex = rgbPart + alphaHex;
        input.value = nextHex;
        syncColorVisuals(nextHex);
        this.mutate(path, nextHex);
      };

      input.oninput = () => {
        let normalized = input.value.trim().toUpperCase();
        if (!normalized.startsWith("#")) normalized = "#" + normalized;
        if (/^#[0-9A-F]{8}$/.test(normalized)) {
          syncColorVisuals(normalized);
          this.mutate(path, normalized);
        }
      };
      input.onchange = () => {
        let normalized = input.value.trim().toUpperCase();
        if (!normalized.startsWith("#")) normalized = "#" + normalized;
        if (/^#[0-9A-F]{6}$/.test(normalized)) normalized += "FF";
        if (/^#[0-9A-F]{8}$/.test(normalized)) {
          input.value = normalized;
          syncColorVisuals(normalized);
          this.mutate(path, normalized);
        }
      };

      const row = document.createElement("div");
      row.className = "color-field";
      const alphaWrap = document.createElement("div");
      alphaWrap.className = "alpha-control-wrap";
      alphaWrap.append(alphaSlider, alphaBadge);
      row.append(swatchBtn, input, alphaWrap);
      return this.buildFieldLabel(meta.label, name, meta.hint, row);
    }

    input.onchange = () => this.mutate(path, input.value);
    return this.buildFieldLabel(meta.label, name, meta.hint, input);
  }

  /** Label a control and expose additional context on hover. */
  private buildFieldLabel(
    title: string,
    keyName: string,
    hint: string | undefined,
    control: HTMLElement,
    inlineSwitch = false,
  ): HTMLElement {
    const wrapper = document.createElement("label");
    wrapper.className = inlineSwitch ? "field field-switch" : "field";

    const head = document.createElement("div");
    head.className = "field-head";

    const titleRow = document.createElement("div");
    titleRow.className = "field-title-row";
    const caption = document.createElement("span");
    caption.className = "field-caption";
    caption.textContent = title;
    const keyTag = document.createElement("code");
    keyTag.className = "field-key";
    keyTag.textContent = keyName;
    titleRow.append(caption);
    caption.title = hint ? `${keyName}: ${hint}` : keyName;
    control.setAttribute("aria-label", title);
    head.append(titleRow);


    wrapper.append(head, control);
    return wrapper;
  }

  private renderNineSliceWorkbench(imagePath: string[]): HTMLElement {
    const box = document.createElement("div");
    box.className = "nine-slice-workbench";

    const currentImg = (objectValue(this.theme, imagePath) ?? {}) as Record<string, unknown>;
    const assetPath = typeof currentImg.asset === "string" ? currentImg.asset : "";
    const crop = (currentImg.crop ?? { left: 0, top: 0, right: 0, bottom: 0 }) as Record<string, number>;
    const slice = (currentImg.slice ?? { xStart: 0.35, xEnd: 0.65, yStart: 0.35, yEnd: 0.65 }) as Record<string, number>;

    const header = document.createElement("div");
    header.className = "nine-slice-header";
    const titleWrap = document.createElement("div");
    titleWrap.innerHTML = `
      <div class="nine-slice-title-row">
        <h5>可视化九宫格与四周裁切画布</h5>
        <span class="nine-slice-badge">拖拽切线实时生效</span>
      </div>
      <p>青色线控制九宫格可拉伸中心区（Slice），琥珀色线控制源图四周透明留白裁切（Crop）。</p>`;
    header.append(titleWrap);

    const presetsBar = document.createElement("div");
    presetsBar.className = "nine-slice-presets";
    const presets: Array<{ label: string; apply: () => void }> = [
      {
        label: "均衡居中 35%–65%",
        apply: () => {
          setValue(this.theme, [...imagePath, "slice"], { xStart: 0.35, xEnd: 0.65, yStart: 0.35, yEnd: 0.65 });
          this.render();
          this.changed();
        },
      },
      {
        label: "细边框 18%–82%",
        apply: () => {
          setValue(this.theme, [...imagePath, "slice"], { xStart: 0.18, xEnd: 0.82, yStart: 0.18, yEnd: 0.82 });
          this.render();
          this.changed();
        },
      },
      {
        label: "宽圆角 28%–72%",
        apply: () => {
          setValue(this.theme, [...imagePath, "slice"], { xStart: 0.28, xEnd: 0.72, yStart: 0.28, yEnd: 0.72 });
          this.render();
          this.changed();
        },
      },
      {
        label: "紧凑中心 42%–58%",
        apply: () => {
          setValue(this.theme, [...imagePath, "slice"], { xStart: 0.42, xEnd: 0.58, yStart: 0.42, yEnd: 0.58 });
          this.render();
          this.changed();
        },
      },
      {
        label: "清零裁切 Crop=0",
        apply: () => {
          setValue(this.theme, [...imagePath, "crop"], { left: 0, top: 0, right: 0, bottom: 0 });
          this.render();
          this.changed();
        },
      },
    ];
    for (const preset of presets) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "nine-slice-preset-btn";
      btn.textContent = preset.label;
      btn.onclick = preset.apply;
      presetsBar.append(btn);
    }

    const stage = document.createElement("div");
    stage.className = "nine-slice-stage";
    const previewUrl = assetPath ? this.assetPreviewProvider?.(assetPath) ?? null : null;

    if (!previewUrl) {
      stage.classList.add("empty");
      const emptyMsg = document.createElement("div");
      emptyMsg.className = "nine-slice-empty";
      emptyMsg.innerHTML = `
        <span>尚未选择或加载贴图资源</span>
        <small>在上方选择已上传图片或点击「上传素材」，即可在此直接拖拽九宫格切线。</small>`;
      stage.append(emptyMsg);
      box.append(header, presetsBar, stage);
      return box;
    }

    const canvasBox = document.createElement("div");
    canvasBox.className = "nine-slice-canvas-box";

    const img = document.createElement("img");
    img.className = "nine-slice-source-img";
    img.src = previewUrl;
    img.alt = assetPath;
    img.draggable = false;

    const overlay = document.createElement("div");
    overlay.className = "nine-slice-overlay";

    const maskTop = document.createElement("div");
    maskTop.className = "crop-mask crop-mask-top";
    const maskBottom = document.createElement("div");
    maskBottom.className = "crop-mask crop-mask-bottom";
    const maskLeft = document.createElement("div");
    maskLeft.className = "crop-mask crop-mask-left";
    const maskRight = document.createElement("div");
    maskRight.className = "crop-mask crop-mask-right";

    const activeCropRect = document.createElement("div");
    activeCropRect.className = "nine-slice-active-rect";

    const centerBox = document.createElement("div");
    centerBox.className = "nine-slice-center-zone";
    centerBox.innerHTML = `<span>CENTER 拉伸区</span>`;

    const handleXStart = document.createElement("div");
    handleXStart.className = "slice-guide slice-guide-v";
    handleXStart.dataset.handle = "xStart";
    handleXStart.title = "拖拽调整水平起点 X-Start";

    const handleXEnd = document.createElement("div");
    handleXEnd.className = "slice-guide slice-guide-v";
    handleXEnd.dataset.handle = "xEnd";
    handleXEnd.title = "拖拽调整水平终点 X-End";

    const handleYStart = document.createElement("div");
    handleYStart.className = "slice-guide slice-guide-h";
    handleYStart.dataset.handle = "yStart";
    handleYStart.title = "拖拽调整垂直起点 Y-Start";

    const handleYEnd = document.createElement("div");
    handleYEnd.className = "slice-guide slice-guide-h";
    handleYEnd.dataset.handle = "yEnd";
    handleYEnd.title = "拖拽调整垂直终点 Y-End";

    const cropLeftLine = document.createElement("div");
    cropLeftLine.className = "crop-guide crop-guide-v";
    cropLeftLine.dataset.handle = "cropLeft";
    cropLeftLine.title = "拖拽调整左侧留白裁切 (Crop Left)";

    const cropRightLine = document.createElement("div");
    cropRightLine.className = "crop-guide crop-guide-v";
    cropRightLine.dataset.handle = "cropRight";
    cropRightLine.title = "拖拽调整右侧留白裁切 (Crop Right)";

    const cropTopLine = document.createElement("div");
    cropTopLine.className = "crop-guide crop-guide-h";
    cropTopLine.dataset.handle = "cropTop";
    cropTopLine.title = "拖拽调整顶部留白裁切 (Crop Top)";

    const cropBottomLine = document.createElement("div");
    cropBottomLine.className = "crop-guide crop-guide-h";
    cropBottomLine.dataset.handle = "cropBottom";
    cropBottomLine.title = "拖拽调整底部留白裁切 (Crop Bottom)";

    activeCropRect.append(centerBox, handleXStart, handleXEnd, handleYStart, handleYEnd);
    overlay.append(
      maskTop,
      maskBottom,
      maskLeft,
      maskRight,
      activeCropRect,
      cropLeftLine,
      cropRightLine,
      cropTopLine,
      cropBottomLine,
    );
    canvasBox.append(img, overlay);
    stage.append(canvasBox);

    const legend = document.createElement("div");
    legend.className = "nine-slice-legend";

    const syncVisualPositions = () => {
      const cLeft = Math.max(0, Math.min(0.45, Number(crop.left ?? 0)));
      const cTop = Math.max(0, Math.min(0.45, Number(crop.top ?? 0)));
      const cRight = Math.max(0, Math.min(0.45, Number(crop.right ?? 0)));
      const cBottom = Math.max(0, Math.min(0.45, Number(crop.bottom ?? 0)));

      const xS = Math.max(0.05, Math.min(0.95, Number(slice.xStart ?? 0.35)));
      const xE = Math.max(xS + 0.01, Math.min(0.95, Number(slice.xEnd ?? 0.65)));
      const yS = Math.max(0.05, Math.min(0.95, Number(slice.yStart ?? 0.35)));
      const yE = Math.max(yS + 0.01, Math.min(0.95, Number(slice.yEnd ?? 0.65)));

      maskTop.style.height = `${cTop * 100}%`;
      maskBottom.style.height = `${cBottom * 100}%`;
      maskLeft.style.top = `${cTop * 100}%`;
      maskLeft.style.bottom = `${cBottom * 100}%`;
      maskLeft.style.width = `${cLeft * 100}%`;
      maskRight.style.top = `${cTop * 100}%`;
      maskRight.style.bottom = `${cBottom * 100}%`;
      maskRight.style.width = `${cRight * 100}%`;

      activeCropRect.style.left = `${cLeft * 100}%`;
      activeCropRect.style.right = `${cRight * 100}%`;
      activeCropRect.style.top = `${cTop * 100}%`;
      activeCropRect.style.bottom = `${cBottom * 100}%`;

      cropLeftLine.style.left = `${cLeft * 100}%`;
      cropRightLine.style.left = `${(1 - cRight) * 100}%`;
      cropTopLine.style.top = `${cTop * 100}%`;
      cropBottomLine.style.top = `${(1 - cBottom) * 100}%`;

      handleXStart.style.left = `${xS * 100}%`;
      handleXEnd.style.left = `${xE * 100}%`;
      handleYStart.style.top = `${yS * 100}%`;
      handleYEnd.style.top = `${yE * 100}%`;

      centerBox.style.left = `${xS * 100}%`;
      centerBox.style.width = `${Math.max(0, xE - xS) * 100}%`;
      centerBox.style.top = `${yS * 100}%`;
      centerBox.style.height = `${Math.max(0, yE - yS) * 100}%`;

      const pct = (v: number) => `${Math.round(v * 100)}%`;
      legend.innerHTML = `
        <span class="legend-item cyan">Slice X: ${pct(xS)} → ${pct(xE)} · Y: ${pct(yS)} → ${pct(yE)}</span>
        <span class="legend-item amber">Crop L:${pct(cLeft)} T:${pct(cTop)} R:${pct(cRight)} B:${pct(cBottom)}</span>`;
    };

    syncVisualPositions();

    const attachDrag = (el: HTMLElement, kind: string) => {
      el.onpointerdown = (downEvent: PointerEvent) => {
        downEvent.preventDefault();
        downEvent.stopPropagation();
        el.setPointerCapture(downEvent.pointerId);
        el.classList.add("dragging");

        const onMove = (moveEvent: PointerEvent) => {
          const fullRect = overlay.getBoundingClientRect();
          const activeRect = activeCropRect.getBoundingClientRect();
          if (fullRect.width <= 0 || fullRect.height <= 0) return;

          if (kind === "xStart" || kind === "xEnd") {
            if (activeRect.width <= 0) return;
            const raw = (moveEvent.clientX - activeRect.left) / activeRect.width;
            if (kind === "xStart") {
              slice.xStart = Number(Math.max(0.05, Math.min(0.9, Math.min(raw, (slice.xEnd ?? 0.65) - 0.01))).toFixed(2));
              setValue(this.theme, [...imagePath, "slice", "xStart"], slice.xStart);
            } else {
              slice.xEnd = Number(Math.max(0.06, Math.min(0.95, Math.max(raw, (slice.xStart ?? 0.35) + 0.01))).toFixed(2));
              setValue(this.theme, [...imagePath, "slice", "xEnd"], slice.xEnd);
            }
          } else if (kind === "yStart" || kind === "yEnd") {
            if (activeRect.height <= 0) return;
            const raw = (moveEvent.clientY - activeRect.top) / activeRect.height;
            if (kind === "yStart") {
              slice.yStart = Number(Math.max(0.05, Math.min(0.9, Math.min(raw, (slice.yEnd ?? 0.65) - 0.01))).toFixed(2));
              setValue(this.theme, [...imagePath, "slice", "yStart"], slice.yStart);
            } else {
              slice.yEnd = Number(Math.max(0.06, Math.min(0.95, Math.max(raw, (slice.yStart ?? 0.35) + 0.01))).toFixed(2));
              setValue(this.theme, [...imagePath, "slice", "yEnd"], slice.yEnd);
            }
          } else if (kind === "cropLeft") {
            const raw = (moveEvent.clientX - fullRect.left) / fullRect.width;
            crop.left = Number(Math.max(0, Math.min(0.45, raw)).toFixed(2));
            setValue(this.theme, [...imagePath, "crop", "left"], crop.left);
          } else if (kind === "cropRight") {
            const raw = (fullRect.right - moveEvent.clientX) / fullRect.width;
            crop.right = Number(Math.max(0, Math.min(0.45, raw)).toFixed(2));
            setValue(this.theme, [...imagePath, "crop", "right"], crop.right);
          } else if (kind === "cropTop") {
            const raw = (moveEvent.clientY - fullRect.top) / fullRect.height;
            crop.top = Number(Math.max(0, Math.min(0.45, raw)).toFixed(2));
            setValue(this.theme, [...imagePath, "crop", "top"], crop.top);
          } else if (kind === "cropBottom") {
            const raw = (fullRect.bottom - moveEvent.clientY) / fullRect.height;
            crop.bottom = Number(Math.max(0, Math.min(0.45, raw)).toFixed(2));
            setValue(this.theme, [...imagePath, "crop", "bottom"], crop.bottom);
          }
          syncVisualPositions();
          this.changed();
        };

        const onUp = (upEvent: PointerEvent) => {
          el.classList.remove("dragging");
          try {
            el.releasePointerCapture(upEvent.pointerId);
          } catch {
            // Ignore
          }
          el.onpointermove = null;
          el.onpointerup = null;
          this.render();
        };

        el.onpointermove = onMove;
        el.onpointerup = onUp;
      };
    };

    attachDrag(handleXStart, "xStart");
    attachDrag(handleXEnd, "xEnd");
    attachDrag(handleYStart, "yStart");
    attachDrag(handleYEnd, "yEnd");
    attachDrag(cropLeftLine, "cropLeft");
    attachDrag(cropRightLine, "cropRight");
    attachDrag(cropTopLine, "cropTop");
    attachDrag(cropBottomLine, "cropBottom");

    box.append(header, presetsBar, stage, legend);
    return box;
  }

  private isColor(node: SchemaNode): boolean {
    return (
      node.type !== "number" &&
      node.type !== "integer" &&
      node.default !== false &&
      node.pattern === "^#[0-9A-F]{8}$"
    );
  }

  private isAssetReference(path: string[]): boolean {
    const joined = path.join(".");
    return (
      joined === "background.asset" ||
      joined === "input.settingsAsset" ||
      joined === "input.attachmentAsset" ||
      joined === "input.micAsset" ||
      joined === "input.sendAsset" ||
      joined === "input.stopAsset" ||
      ((path[0] === "chat" || path[0] === "input") && path.at(-1) === "asset") ||
      (path[0] === "typography" && path.length === 2)
    );
  }
}
