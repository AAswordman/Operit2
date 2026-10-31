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

// src/ui/summary.ui.ts
var summary_ui_exports = {};
__export(summary_ui_exports, {
  default: () => Summary
});
module.exports = __toCommonJS(summary_ui_exports);

// src/model.ts
function decodeXml(value) {
  return value.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/&quot;/g, '"').replace(/&apos;/g, "'");
}
function metadata(value) {
  if (value === void 0 || value.length === 0) return null;
  const text = /^\s*\{\s*&quot;/.test(value) ? decodeXml(value) : value;
  const decoded = JSON.parse(text);
  return decoded !== null && typeof decoded === "object" && !Array.isArray(decoded) ? decoded : null;
}
function lineStats(diff) {
  let additions = 0;
  let deletions = 0;
  for (const line of diff.split("\n")) {
    if (line.startsWith("+") && !line.startsWith("+++")) additions++;
    if (line.startsWith("-") && !line.startsWith("---")) deletions++;
  }
  return { additions, deletions };
}
function envelopeDiffs(text) {
  const changes = [];
  for (const match of text.matchAll(/<file-diff\b([^>]*)>([\s\S]*?)<\/file-diff>/gi)) {
    const path = /\bpath="([^"]*)"/i.exec(match[1]);
    const cdata = /<!\[CDATA\[([\s\S]*?)\]\]>/i.exec(match[2]);
    changes.push({
      path: path === null ? "" : decodeXml(path[1]),
      diffContent: decodeXml((cdata === null ? match[2] : cdata[1]).trim())
    });
  }
  return changes;
}
function reviewForMessage(message) {
  const files = /* @__PURE__ */ new Map();
  for (const part of message.parts) {
    if (part.kind !== "tool_result") continue;
    const display = metadata(part.attributes.display);
    if (display === null || display.resultRenderer !== "fileDiff" || display.label === null || typeof display.label !== "object" || Array.isArray(display.label) || Object.values(display.label).some((value) => typeof value !== "string")) continue;
    let changes;
    if (part.attributes.result_data === void 0 || part.attributes.result_data === "") {
      changes = envelopeDiffs(part.content);
    } else {
      const data = metadata(part.attributes.result_data);
      const operation = data?.operation;
      if (operation === null || typeof operation !== "object" || Array.isArray(operation)) throw new Error("File diff requires a typed operation");
      const typed = operation;
      if (typeof typed.path !== "string" || typeof data?.diffContent !== "string" || typed.details !== void 0 && typed.details !== null && typeof typed.details !== "string") throw new Error("Invalid typed file diff");
      changes = data.diffContent.length === 0 ? [] : [{ path: typed.path, diffContent: data.diffContent }];
    }
    for (const file of changes) {
      const previous = files.get(file.path);
      files.set(file.path, previous === void 0 ? file.diffContent : previous + "\n" + file.diffContent);
    }
  }
  return [...files].map(([path, diffContent]) => ({ path, diffContent, ...lineStats(diffContent) }));
}
function fileIdentity(path) {
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  return {
    name: index < 0 || index === normalized.length - 1 ? path : normalized.slice(index + 1),
    directory: index < 0 ? "" : normalized.slice(0, index + 1)
  };
}

// src/ui/summary.ui.ts
var ROUTE = "toolpkg:com.operit.change_review:ui:workspace";
function Summary(ctx) {
  const [message] = ctx.useState("message", null);
  const [chatId] = ctx.useState("chatId", "");
  const [locale] = ctx.useState("locale", "zh");
  const [streaming] = ctx.useState("isStreaming", false);
  const [expanded, setExpanded] = ctx.useState("files-expanded", true);
  const { UI, Modifier: M } = ctx;
  const c = ctx.MaterialTheme.colorScheme;
  const en = locale.startsWith("en");
  const dark = ctx.Theme.getCurrent().brightness === "dark";
  const addedColor = dark ? "#FF8DD3A8" : "#FF267344";
  const removedColor = dark ? "#FFF0A5A0" : "#FFB23D38";
  const files = ctx.useMemo("review", () => message === null || message.sender !== "ai" || streaming ? [] : reviewForMessage(message), [message, streaming]);
  if (message === null || message.sender !== "ai" || streaming || files.length === 0) return UI.Spacer({ width: 0, height: 0 });
  const totals = files.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 });
  function open(file) {
    return __async(this, null, function* () {
      yield ctx.navigate(ROUTE, {
        chatId,
        messageTimestamp: message.timestamp,
        variantIndex: message.variantIndex,
        files: file === void 0 ? files : [file, ...files.filter((item) => item.path !== file.path)]
      });
    });
  }
  function count(value, added) {
    return UI.Text({
      inheritStyle: true,
      text: `${added ? "+" : "−"}${value}`,
      color: value === 0 ? c.onSurfaceVariant : added ? addedColor : removedColor,
      fontWeight: "600",
      fontSize: 12,
      fontFamily: "monospace"
    });
  }
  function fileRow(file) {
    const { name, directory } = fileIdentity(file.path);
    return UI.InkWell(
      { shape: { cornerRadius: 6 }, onClick: () => open(file) },
      UI.Tooltip(
        { message: `${en ? "View changes" : "查看变更"} · ${file.path}`, waitMillis: 400 },
        UI.Row({ modifier: M.padding(12, 10), verticalAlignment: "center" }, [
          UI.Icon({ name: "insert_drive_file_outlined", size: 16, tint: c.onSurfaceVariant }),
          UI.Spacer({ width: 10 }),
          UI.RichText({ weight: 1, maxLines: 1, overflow: "ellipsis", spans: [
            { text: name, fontSize: 12.5, fontWeight: "600", color: c.onSurface },
            ...directory.length === 0 ? [] : [
              { text: "  " },
              { text: directory, fontSize: 11.5, color: c.onSurfaceVariant }
            ]
          ] }),
          UI.Spacer({ width: 10 }),
          count(file.additions, true),
          UI.Spacer({ width: 10 }),
          count(file.deletions, false),
          UI.Spacer({ width: 8 }),
          UI.Icon({ name: "chevron_right_rounded", size: 14, tint: c.onSurfaceVariant })
        ])
      )
    );
  }
  const header = UI.InkWell({
    shape: { topLeft: 10, topRight: 10, bottomLeft: expanded ? 0 : 10, bottomRight: expanded ? 0 : 10 },
    onClick: () => setExpanded(!expanded)
  }, UI.Row({ modifier: M.padding(12, 10), verticalAlignment: "center" }, [
    UI.Icon({ name: "difference_outlined", size: 15, tint: c.onSurfaceVariant }),
    UI.Spacer({ width: 8 }),
    UI.FlowRow({ weight: 1, spacing: 8, runSpacing: 4, itemVerticalAlignment: "center" }, [
      UI.Text({ maxLines: 1, overflow: "ellipsis", text: en ? `Edited ${files.length} ${files.length === 1 ? "file" : "files"}` : `已编辑 ${files.length} 个文件`, style: "labelMedium", color: c.onSurface, fontWeight: "600" }),
      count(totals.additions, true),
      count(totals.deletions, false)
    ]),
    UI.Spacer({ width: 8 }),
    UI.Tooltip(
      { message: en ? "Undo" : "撤销", waitMillis: 400 },
      UI.TextButton({
        compact: true,
        contentColor: c.onSurfaceVariant,
        contentPadding: { horizontal: 6, vertical: 5 },
        onClick: () => ({ type: "toolpkg.message.rollback" })
      }, [
        UI.Icon({ name: "undo_rounded", size: 15 }),
        UI.Spacer({ width: 6 }),
        UI.Text({ inheritStyle: true, text: en ? "Undo" : "撤销", fontSize: 11.5 })
      ])
    ),
    UI.Spacer({ width: 4 }),
    UI.InkWell(
      { shape: { cornerRadius: 6 }, onClick: () => open() },
      UI.Box(
        { modifier: M.padding(6, 5) },
        UI.Text({ inheritStyle: true, text: en ? "Review" : "查看变更", fontSize: 12, fontWeight: "500", color: c.onSurface })
      )
    ),
    UI.Spacer({ width: 6 }),
    UI.Icon({ name: "keyboard_arrow_down", size: 16, tint: c.onSurfaceVariant, turns: expanded ? 0.5 : 0, durationMillis: 200 })
  ]));
  const body = UI.Box(
    { animation: { animateContentSize: true, durationMillis: 220, curve: "easeOutCubic", alignment: "topCenter" } },
    expanded ? UI.Column({ fillMaxWidth: true }, [
      UI.Divider({ height: 1, thickness: 0.8, color: c.outlineVariant.copy({ alpha: 0.55 }) }),
      UI.Box(
        { modifier: M.heightIn(0, 240) },
        UI.ScrollArea(
          { thumbVisibility: files.length > 5 },
          UI.Column({ modifier: M.padding(4, 2), fillMaxWidth: true }, files.map(fileRow))
        )
      )
    ]) : UI.Spacer({ height: 0 })
  );
  return UI.Box(
    { modifier: M.fillMaxWidth().padding(12, 6, 12, 4) },
    UI.Card(
      {
        themeSurface: "card",
        material: false,
        containerColor: c.surfaceContainer,
        shape: { cornerRadius: 10 },
        border: { color: c.outlineVariant, alpha: 0.65, width: 0.8 }
      },
      UI.Column({ fillMaxWidth: true }, [header, body])
    )
  );
}
