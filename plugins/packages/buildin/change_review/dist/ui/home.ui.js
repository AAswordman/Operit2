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
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};

// src/ui/home.ui.ts
var home_ui_exports = {};
__export(home_ui_exports, {
  default: () => Home
});
module.exports = __toCommonJS(home_ui_exports);
function Home(ctx) {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  const [locale] = ctx.useState("locale", "zh");
  const en = locale.startsWith("en");
  function open() {
    return __async(this, null, function* () {
      yield ctx.navigate("toolpkg:com.operit.change_review:ui:workspace");
    });
  }
  return UI.InkWell(
    { onClick: open, shape: { cornerRadius: 8 } },
    UI.Row({ modifier: M.padding(12, 16), verticalAlignment: "center" }, [
      UI.Icon({ name: "difference_outlined", size: 18, tint: c.onSurfaceVariant }),
      UI.Spacer({ width: 10 }),
      UI.Column({ weight: 1, horizontalAlignment: "start" }, [
        UI.Text({ text: en ? "Code changes" : "代码变更", style: "bodyMedium", fontWeight: "500", color: c.onSurface }),
        UI.Spacer({ height: 4 }),
        UI.Text({ text: en ? "Review file changes" : "查看文件改动与对比", style: "bodySmall", color: c.onSurfaceVariant })
      ]),
      UI.Spacer({ width: 12 }),
      UI.Icon({ name: "chevron_right_rounded", size: 16, tint: c.onSurfaceVariant })
    ])
  );
}
