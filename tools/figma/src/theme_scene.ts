import {
  clone,
  getPath,
  validateTheme,
  validateDocument,
  type DesignAsset,
  type DesignDocument,
  type DesignNode,
  type Theme,
} from "./bridge.ts";

/** Editable starting kit. Live screen captures are supplied by the runtime exporter. */
export function themeDocument(
  theme: Theme,
  assets: DesignAsset[],
  name = "Operit UI",
): DesignDocument {
  validateTheme(theme);
  const scenes: DesignNode[] = [];
  for (const [index, mode] of ["light", "dark"].entries()) {
    for (const [device, width, height] of [
      ["Desktop", 1280, 800],
      ["Phone", 390, 844],
    ] as const) {
      let next = 0;
      const id = () => `${mode}-${device}-${++next}`;
      const palette = (key: string) =>
        getPath(theme, `palette.${mode}.${key}`) as string;
      const text = (
        name: string,
        value: string,
        x: number,
        y: number,
        w: number,
        size = 14,
        color = "onSurface",
      ): DesignNode => ({
        id: id(),
        name,
        type: "text",
        x,
        y,
        width: w,
        height: size * 1.6,
        text: value,
        fontSize: size,
        fill: palette(color),
        fontFamily: "Noto Sans SC",
        bindings: [{ path: `palette.${mode}.${color}`, property: "textColor" }],
      });
      const panel = (
        name: string,
        x: number,
        y: number,
        w: number,
        h: number,
        color: string,
      ): DesignNode => ({
        id: id(),
        name,
        type: "frame",
        x,
        y,
        width: w,
        height: h,
        fill: palette(color),
        children: [],
        bindings: [{ path: `palette.${mode}.${color}`, property: "fill" }],
      });
      const root = panel(`${device} / ${mode}`, 0, 0, width, height, "surface");
      root.id = `${mode}-${device}`;
      root.source = {
        kind: "theme-kit",
        mode,
        device,
        description: "可编辑组件起点；精确布局请采集运行中的应用",
      };
      root.y = index * 1000;
      root.x = device === "Phone" ? 1440 : 0;
      root.clipsContent = true;
      if (getPath(theme, "background.asset"))
        root.children!.push({
          id: id(),
          name: "背景图片",
          type: "image",
          x: 0,
          y: 0,
          width,
          height,
          asset: getPath(theme, "background.asset") as string,
          opacity: getPath(theme, "background.opacity") as number,
          bindings: [{ path: "background.asset", property: "image" }],
        });
      const sidebar = device === "Desktop" ? 280 : 0;
      if (sidebar) {
        const nav = panel(
          "导航侧栏",
          0,
          0,
          sidebar,
          height,
          "surfaceContainerLow",
        );
        nav.source = { component: "ThemeDesktopSidebarSurface" };
        nav.children!.push(
          text("品牌", "Operit", 24, 30, 220, 26),
          text("新建会话", "＋ 新建会话", 24, 92, 220, 16, "primary"),
        );
        ["今天", "设计协作", "工具与插件", "工作区"].forEach((label, i) =>
          nav.children!.push(
            text(`导航 / ${label}`, label, 24, 170 + i * 54, 220),
          ),
        );
        root.children!.push(nav);
      }
      const chatWidth = width - sidebar;
      const top = panel("对话顶栏", sidebar, 0, chatWidth, 64, "surface");
      top.source = { component: "ThemeTopBarChrome" };
      top.children!.push(
        text("会话标题", "新的会话", 20, 20, chatWidth - 40, 18),
      );
      root.children!.push(top);
      for (const [role, y, label] of [
        ["user", 112, "帮我整理一下今天的工作"],
        ["ai", 230, "可以。我们先梳理任务，再逐项推进。"],
      ] as const) {
        const w = Math.min(chatWidth - 48, role === "user" ? 340 : 540);
        const bubble = panel(
          role === "user" ? "用户气泡" : "AI 气泡",
          sidebar + (role === "user" ? chatWidth - w - 24 : 24),
          y,
          w,
          88,
          role === "user" ? "primaryContainer" : "surfaceContainerLow",
        );
        bubble.fill =
          (getPath(theme, `chat.${role}.color`) as string | null) ??
          bubble.fill;
        bubble.radius = getPath(theme, `chat.${role}.roundedCorners`) ? 16 : 0;
        bubble.paddingLeft = getPath(
          theme,
          `chat.${role}.padding.left`,
        ) as number;
        bubble.paddingRight = getPath(
          theme,
          `chat.${role}.padding.right`,
        ) as number;
        bubble.bindings = [
          {
            path: `chat.${role}.color`,
            fallback: `palette.${mode}.${role === "user" ? "primaryContainer" : "surfaceContainerLow"}`,
            property: "fill",
          },
          {
            path: `chat.${role}.roundedCorners`,
            property: "cornerRadius",
            boolean: true,
          },
          { path: `chat.${role}.padding.left`, property: "paddingLeft" },
          { path: `chat.${role}.padding.right`, property: "paddingRight" },
        ];
        bubble.source = { component: "BubbleSurface", role };
        const image = getPath(theme, `chat.${role}.image.asset`);
        if (image)
          bubble.children!.push({
            id: id(),
            name: "气泡贴图",
            type: "image",
            x: 0,
            y: 0,
            width: w,
            height: 88,
            asset: image as string,
            bindings: [{ path: `chat.${role}.image.asset`, property: "image" }],
          });
        const labelNode = text(
          "消息文字",
          label,
          bubble.paddingLeft!,
          20,
          w - bubble.paddingLeft! - bubble.paddingRight!,
          15,
          role === "user" ? "onPrimaryContainer" : "onSurface",
        );
        labelNode.fill =
          (getPath(theme, `chat.${role}.textColor`) as string | null) ??
          labelNode.fill;
        labelNode.bindings = [
          {
            path: `chat.${role}.textColor`,
            fallback: `palette.${mode}.${role === "user" ? "onPrimaryContainer" : "onSurface"}`,
            property: "textColor",
          },
        ];
        bubble.children!.push(labelNode);
        root.children!.push(bubble);
      }
      const composer = panel(
        "消息输入框",
        sidebar + 16,
        height - 130,
        chatWidth - 32,
        106,
        "surfaceContainerHigh",
      );
      composer.source = { component: "ThemeComposerChrome" };
      composer.fill =
        (getPath(theme, "input.color") as string | null) ?? composer.fill;
      composer.radius = getPath(theme, "input.roundedCorners") ? 20 : 0;
      composer.bindings = [
        {
          path: "input.color",
          fallback: `palette.${mode}.surfaceContainerHigh`,
          property: "fill",
        },
        {
          path: "input.roundedCorners",
          property: "cornerRadius",
          boolean: true,
        },
      ];
      const inputImage = getPath(theme, "input.image.asset");
      if (inputImage)
        composer.children!.push({
          id: id(),
          name: "输入框贴图",
          type: "image",
          x: 0,
          y: 0,
          width: composer.width,
          height: 106,
          asset: inputImage as string,
          bindings: [{ path: "input.image.asset", property: "image" }],
        });
      composer.children!.push(
        text(
          "输入提示",
          "输入消息…",
          16,
          14,
          composer.width - 32,
          14,
          "onSurfaceVariant",
        ),
      );
      const icons = [
        ["settings", "设置"],
        ["attachment", "附件"],
        ["mic", "语音"],
        ["send", "发送"],
        ["stop", "停止"],
      ] as const;
      icons.forEach(([key, label], i) => {
        const size = getPath(theme, `input.${key}AssetSize`) as number;
        const asset = getPath(theme, `input.${key}Asset`) as string | null;
        const icon: DesignNode = asset
          ? {
              id: id(),
              name: label,
              type: "image",
              x: 16 + i * 52,
              y: 62,
              width: size,
              height: size,
              asset,
            }
          : {
              ...text(label, label, 16 + i * 52, 62, 40, 12, "primary"),
              width: size,
              height: size,
            };
        icon.bindings = [
          { path: `input.${key}AssetSize`, property: "width" },
          { path: `input.${key}AssetSize`, property: "height" },
        ];
        if (asset)
          icon.bindings.push({ path: `input.${key}Asset`, property: "image" });
        composer.children!.push(icon);
      });
      root.children!.push(composer);
      scenes.push(root);
    }
  }
  return validateDocument({
    format: "operit-design",
    version: 1,
    name,
    origin: "theme-studio",
    theme: clone(theme),
    assets: clone(assets),
    scenes,
    warnings: [
      "主题组件起点使用示意布局。工具结果、插件界面与实际页面请用运行时采集；不会从截图推测图层。",
      "九宫格、液态玻璃和动态交互以 Flutter 预览为准。原始主题字段和素材会随设计文件完整保留。",
    ],
  });
}
