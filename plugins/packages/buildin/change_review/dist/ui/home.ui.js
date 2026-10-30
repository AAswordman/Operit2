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

// src/ui/home.ui.ts
var home_ui_exports = {};
__export(home_ui_exports, {
  default: () => Home
});
module.exports = __toCommonJS(home_ui_exports);
function Home(ctx) {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  function open() {
    return { type: "toolpkg.ui.open", placement: "workspace", routeId: "toolpkg:com.operit.change_review:ui:workspace", instanceKey: "change-review", input: {} };
  }
  return UI.OutlinedCard(
    { containerColor: c.surfaceContainerLow, elevation: 0, shape: { cornerRadius: 10 }, border: { width: 1, color: c.outlineVariant } },
    UI.Row({ modifier: M.padding(14, 12), verticalAlignment: "center" }, [
      UI.Box({ width: 36, height: 36, contentAlignment: "center", modifier: M.background(c.primary.copy({ alpha: 0.15 }), { cornerRadius: 8 }).border(1, c.primary.copy({ alpha: 0.3 }), { cornerRadius: 8 }) }, UI.Icon({ name: "difference_outlined", size: 20, tint: c.primary })),
      UI.Spacer({ width: 12 }),
      UI.Column({ weight: 1, horizontalAlignment: "start" }, [
        UI.Text({ text: "代码变更审查", style: "bodyMedium", fontWeight: "600", color: c.onSurface }),
        UI.Spacer({ height: 2 }),
        UI.Text({ text: "查看工作区代码改动、行级对比与文件审查", style: "bodySmall", fontSize: 11.5, color: c.onSurfaceVariant.copy({ alpha: 0.8 }) })
      ]),
      UI.Spacer({ width: 8 }),
      UI.FilledTonalButton({ onClick: open, containerColor: c.surfaceContainerHigh, contentColor: c.onSurface, contentPadding: { horizontal: 12, vertical: 8 }, shape: { cornerRadius: 7 }, border: { width: 1, color: c.outlineVariant.copy({ alpha: 0.4 }) } }, UI.Row({ verticalAlignment: "center" }, [
        UI.Text({ text: "查看变更", inheritStyle: true, fontWeight: "600", fontSize: 12, color: c.onSurface }),
        UI.Spacer({ width: 4 }),
        UI.Icon({ name: "arrow_forward_rounded", size: 14, tint: c.onSurfaceVariant })
      ]))
    ])
  );
}
