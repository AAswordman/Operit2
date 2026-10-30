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

// src/icons.ts
var icons = {
  "VsCode": '<svg role="img" viewBox="0 0 24 24" fill="#007ACC" xmlns="http://www.w3.org/2000/svg">\n<path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>\n</svg>',
  "Terminal": '<svg role="img" viewBox="0 0 24 24" fill="#5391FE" xmlns="http://www.w3.org/2000/svg">\n<path d="M24 18.064c0 1.34-1.077 2.42-2.408 2.42H2.408C1.077 20.484 0 19.404 0 18.064V5.936C0 4.596 1.077 3.516 2.408 3.516h19.184C22.923 3.516 24 4.596 24 5.936v12.128zm-11.455-6.07l-6.86-5.474a.81.81 0 0 0-1.12.115.815.815 0 0 0 .114 1.127l5.966 4.76-5.966 4.761a.813.813 0 0 0-.114 1.127.81.81 0 0 0 1.12.115l6.86-5.474a.816.816 0 0 0 0-1.057zm7.42 4.417h-6.286a.81.81 0 1 0 0 1.621h6.286a.81.81 0 1 0 0-1.621z"/>\n</svg>',
  "TreeHierarchy": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<rect x="9" y="3" width="6" height="5" rx="1"/>\n<rect x="3" y="15" width="6" height="5" rx="1"/>\n<rect x="15" y="15" width="6" height="5" rx="1"/>\n<path d="M12 8v4M6 15v-3h12v3"/>\n</svg>',
  "LangPython": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M11.91 2c-5.06 0-4.73 2.19-4.73 2.19l.01 2.27h4.81v.68H5.21S2 6.77 2 11.88s2.8 5 2.8 5h1.67v-2.33s-.09-2.8 2.76-2.8h4.74s2.66.04 2.66-2.6V4.6s.4-2.6-4.72-2.6zm-2.58 1.48a.86.86 0 1 1 0 1.72.86.86 0 0 1 0-1.72z" fill="#3776AB"/>\n<path d="M12.09 22c5.06 0 4.73-2.19 4.73-2.19l-.01-2.27h-4.81v-.68h6.78s3.21.37 3.21-4.74-2.8-5-2.8-5h-1.67v2.33s.09 2.8-2.76 2.8H10.1s-2.66-.04-2.66 2.6V19.4s-.4 2.6 4.72 2.6zm2.58-1.48a.86.86 0 1 1 0-1.72.86.86 0 0 1 0 1.72z" fill="#FFD43B"/>\n</svg>',
  "LangRust": '<svg viewBox="0 0 24 24" fill="#DEA584" xmlns="http://www.w3.org/2000/svg">\n<path d="M23.83 12c0-.29-.02-.57-.05-.84l-1.92-.32a9.98 9.98 0 0 0-.58-1.41l1.33-1.44a11.96 11.96 0 0 0-1.23-1.9l-1.84.66a9.97 9.97 0 0 0-1.08-1.08l.66-1.84a11.97 11.97 0 0 0-1.9-1.23l-1.44 1.33a10.03 10.03 0 0 0-1.41-.59l-.32-1.92A11.75 11.75 0 0 0 12 .17c-.29 0-.57.02-.84.05l-.32 1.92a9.98 9.98 0 0 0-1.41.58l-1.44-1.33a11.96 11.96 0 0 0-1.9 1.23l.66 1.84a9.98 9.98 0 0 0-1.08 1.08l-1.84-.66a11.97 11.97 0 0 0-1.23 1.9l1.33 1.44c-.23.45-.43.93-.59 1.41l-1.92.32a11.77 11.77 0 0 0-.05.85c0 .28.02.56.05.84l1.92.32c.16.49.36.96.59 1.41l-1.33 1.44a11.96 11.96 0 0 0 1.23 1.9l1.84-.66c.34.4.7.76 1.08 1.08l-.66 1.84a11.97 11.97 0 0 0 1.9 1.23l1.44-1.33c.45.23.93.43 1.41.59l.32 1.92c.28.03.56.05.84.05.29 0 .57-.02.84-.05l.32-1.92a10.02 10.02 0 0 0 1.41-.59l1.44 1.33a11.96 11.96 0 0 0 1.9-1.23l-.66-1.84a9.98 9.98 0 0 0 1.08-1.08l1.84.66a11.97 11.97 0 0 0 1.23-1.9l-1.33-1.44c.23-.45.43-.93.59-1.41l1.92-.32c.03-.28.05-.56.05-.84zm-11.83 6.64a6.64 6.64 0 1 1 0-13.28 6.64 6.64 0 0 1 0 13.28zm-2.02-9.61h2.24c1.17 0 1.91.56 1.91 1.48 0 .76-.48 1.28-1.24 1.43l1.43 2.7h-1.28l-1.28-2.48h-.6v2.48H9.98V9.03zm1.18 2.11h.96c.55 0 .9-.25.9-.66 0-.42-.35-.66-.9-.66h-.96v1.32z"/>\n</svg>',
  "LangC": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#00599C"/>\n<path d="M14.8 8.8a4.2 4.2 0 0 0-5.6 0 4 4 0 0 0 0 6.4 4.2 4.2 0 0 0 5.6 0" stroke="#FFFFFF" stroke-width="2.4" stroke-linecap="round"/>\n</svg>',
  "LangCpp": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#004482"/>\n<path d="M12 9.2a3.5 3.5 0 0 0-4.8 0 3.4 3.4 0 0 0 0 5.6 3.5 3.5 0 0 0 4.8 0" stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round"/>\n<path d="M14 12h2.5M15.25 10.75v2.5M17.5 12h2.5M18.75 10.75v2.5" stroke="#659AD2" stroke-width="1.5" stroke-linecap="round"/>\n</svg>',
  "LangDart": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4.1 3.5L3 13.2l5.7 5.7 8.2-8.2L4.1 3.5z" fill="#0175C2"/>\n<path d="M16.9 10.7l-8.2 8.2 2.1 2.1c1.2 1.2 3.1 1.2 4.3 0l5.8-5.8-4-4.5z" fill="#02569B"/>\n<path d="M12.9 2.5c-1.2-1.2-3.1-1.2-4.3 0L3 8.1l1.1 5.1 12.8-2.5-4-8.2z" fill="#00B4AB"/>\n<path d="M16.9 10.7l4 4.5 1.1-5.1-5.1-7.6-4 8.2 4 0z" fill="#29B6F6"/>\n</svg>',
  "LangTs": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#3178C6"/>\n<path d="M11 9H5v2.2h2V18h2.3v-6.8H11V9zm3.8 4.7c.8.6 1.7.9 2.7.9 1 0 1.6-.4 1.6-1 0-.6-.5-.9-1.8-1.4-1.7-.6-2.8-1.5-2.8-3 0-1.8 1.4-3.2 3.6-3.2 1.3 0 2.3.4 3.1 1l-1 1.8c-.7-.5-1.4-.7-2.1-.7-.8 0-1.3.4-1.3.9 0 .6.5.8 1.7 1.3 1.9.7 2.9 1.6 2.9 3.2 0 1.9-1.4 3.3-3.8 3.3-1.6 0-2.8-.5-3.8-1.2l1-1.9z" fill="#FFFFFF"/>\n</svg>',
  "LangJs": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#F7DF1E"/>\n<path d="M7.5 17.5c1 .6 2.1.9 3.2.9 1.8 0 2.8-.9 2.8-2.5V9H11v6.9c0 .6-.3.9-.9.9-.4 0-.8-.1-1.2-.3l-1.4 1zM15 17.2c1 .6 2.1 1 3.4 1 2.2 0 3.6-1.1 3.6-2.8 0-1.6-1-2.4-2.6-3-1.1-.5-1.6-.8-1.6-1.3 0-.5.4-.8 1.2-.8.8 0 1.5.3 2.1.7l1.1-1.8c-.9-.6-2-1-3.2-1-2.1 0-3.5 1.2-3.5 2.8 0 1.5 1 2.4 2.5 3 1.1.5 1.7.8 1.7 1.4 0 .6-.5.9-1.4.9-.9 0-1.8-.3-2.5-.8l-1.3 1.9z" fill="#000000"/>\n</svg>',
  "LangGo": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="5" fill="#00ADD8"/>\n<path d="M10.8 12.3H8.3v1.8h4.5c-.4 2.3-2 3.6-4.5 3.6-2.9 0-5.1-2.1-5.1-5.4s2.2-5.4 5.1-5.4c1.8 0 3.1.8 3.9 1.8l-1.6 1.4c-.6-.7-1.3-1.2-2.3-1.2-1.7 0-3 1.4-3 3.4s1.3 3.4 3 3.4c1.2 0 2-.6 2.4-1.5H10.8v-1.9zm8.4-.3c0 3.3-2.3 5.7-5.5 5.7s-5.5-2.4-5.5-5.7 2.3-5.7 5.5-5.7 5.5 2.4 5.5 5.7zm-2.4 0c0-2-1.3-3.6-3.1-3.6s-3.1 1.6-3.1 3.6 1.3 3.6 3.1 3.6 3.1-1.6 3.1-3.6z" fill="#FFFFFF"/>\n</svg>',
  "LangJava": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 10h12v6a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4v-6zm12 2h2a2 2 0 0 1 2 2v0a2 2 0 0 1-2 2h-2v-4z" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>\n<path d="M7 6c0-1.5 1-2.5 1-4M10 6c0-1.5 1-2.5 1-4M13 6c0-1.5 1-2.5 1-4" stroke="#5382A1" stroke-width="1.8" stroke-linecap="round"/>\n<path d="M2 21h18" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>\n</svg>',
  "LangKotlin": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#7F52FF"/>\n<path d="M4 20L12 12 4 4v16zM12 12l8 8H12v-8zM12 12l8-8H12v8z" fill="#FFFFFF" fill-opacity="0.85"/>\n</svg>',
  "LangSwift": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="5" fill="#F05138"/>\n<path d="M19.2 17.5c-3.1 2.2-7.3 1.8-10.4-.9 4.2 1.4 8.7-.3 10.4-.9-2.3-1.6-4.5-4-5.2-6.8 1.8 1.4 4.1 2 6.4 1.4-1.9-2.6-4.9-5-8.5-5.8 4.2.9 7.8 3.5 9.7 7.2-.6-1.5-1.5-2.8-2.6-3.9 3.2 2.7 4.1 7.1.2 9.7z" fill="#FFFFFF"/>\n</svg>',
  "LangHtml": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#E34F26"/>\n<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#EF652A"/>\n<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-2.1l-.1-1.3H8.3l.3 3.6h3.4v-2.3zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>\n</svg>',
  "LangCss": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#1572B6"/>\n<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#33A9DC"/>\n<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-3.5l.3 3.6 3.2.9v-2.4l-1.4-.4-.1-1.7H12v-0zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>\n</svg>',
  "LangJson": '<svg viewBox="0 0 24 24" fill="none" stroke="#CB85E8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M9 5c-1.5 0-2 1-2 2v3c0 1-1 2-2 2 1 0 2 1 2 2v3c0 1 .5 2 2 2M15 5c1.5 0 2 1 2 2v3c0 1 1 2 2 2-1 0-2 1-2 2v3c0 1-.5 2-2 2"/>\n</svg>',
  "LangYaml": '<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>\n</svg>',
  "LangMarkdown": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="16" x="1" y="4" rx="3" stroke="#79B8FF" stroke-width="2"/>\n<path d="M4 15V9l2.5 3L9 9v6M13 12h3M14.5 9.5v5.5m0 0l-2-2m2 2l2-2" stroke="#79B8FF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>\n</svg>',
  "LangShell": '<svg viewBox="0 0 24 24" fill="none" stroke="#5391FE" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<polyline points="4 17 10 11 4 5"/>\n<line x1="12" y1="19" x2="20" y2="19"/>\n</svg>',
  "LangSql": '<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<ellipse cx="12" cy="5" rx="9" ry="3"/>\n<path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>\n</svg>',
  "LangDocker": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M2.5 12.5h19c0 4.5-3.5 8-8 8-4 0-7.2-2.8-7.9-6.5l-3.1-1.5z" fill="#0db7ed"/>\n<rect x="5" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="8" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="11" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="8" y="6" width="2" height="2" fill="#0db7ed"/>\n<rect x="11" y="6" width="2" height="2" fill="#0db7ed"/>\n<rect x="14" y="9" width="2" height="2" fill="#0db7ed"/>\n</svg>',
  "LangGit": '<svg viewBox="0 0 24 24" fill="none" stroke="#F05032" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<circle cx="6" cy="6" r="3"/>\n<circle cx="6" cy="18" r="3"/>\n<circle cx="18" cy="9" r="3"/>\n<line x1="6" y1="9" x2="6" y2="15"/>\n<path d="M6 12a9 9 0 0 0 9-3"/>\n</svg>',
  "LangLock": '<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>\n<path d="M7 11V7a5 5 0 0 1 10 0v4"/>\n</svg>',
  "LangDefault": '<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>\n<polyline points="14 2 14 8 20 8"/>\n</svg>'
};
function fileIcon(name) {
  const lower = name.toLowerCase();
  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) return icons.LangDocker;
  if (lower.startsWith(".git")) return icons.LangGit;
  const extension = name.indexOf(".") < 0 ? "" : name.split(".").pop().toLowerCase();
  switch (extension) {
    case "c":
    case "h":
      return icons.LangC;
    case "cpp":
    case "cc":
    case "cxx":
    case "hpp":
    case "hxx":
      return icons.LangCpp;
    case "py":
    case "pyw":
    case "ipynb":
      return icons.LangPython;
    case "rs":
      return icons.LangRust;
    case "dart":
      return icons.LangDart;
    case "ts":
    case "tsx":
    case "mts":
    case "cts":
      return icons.LangTs;
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return icons.LangJs;
    case "go":
      return icons.LangGo;
    case "java":
    case "class":
    case "jar":
      return icons.LangJava;
    case "kt":
    case "kts":
      return icons.LangKotlin;
    case "swift":
      return icons.LangSwift;
    case "html":
    case "htm":
      return icons.LangHtml;
    case "css":
    case "scss":
    case "less":
      return icons.LangCss;
    case "json":
    case "jsonc":
      return icons.LangJson;
    case "yaml":
    case "yml":
    case "toml":
      return icons.LangYaml;
    case "md":
    case "markdown":
      return icons.LangMarkdown;
    case "sh":
    case "bash":
    case "zsh":
    case "ps1":
    case "bat":
    case "cmd":
      return icons.LangShell;
    case "sql":
      return icons.LangSql;
    case "lock":
      return icons.LangLock;
    default:
      return icons.LangDefault;
  }
}

// src/ui/summary.ui.ts
var ROUTE = "toolpkg:com.operit.change_review:ui:workspace";
var GREEN = "#FF3FB950";
var RED = "#FFF85149";
function Summary(ctx) {
  const [message] = ctx.useState("message", null);
  const [chatId] = ctx.useState("chatId", "");
  const [locale] = ctx.useState("locale", "zh");
  const [streaming] = ctx.useState("isStreaming", false);
  const [expanded, setExpanded] = ctx.useState("files-expanded", true);
  const { UI, Modifier: M } = ctx;
  const c = ctx.MaterialTheme.colorScheme;
  const en = locale.startsWith("en");
  const files = ctx.useMemo("review", () => message === null || message.sender !== "ai" || streaming ? [] : reviewForMessage(message), [message, streaming]);
  if (message === null || message.sender !== "ai" || streaming || files.length === 0) return UI.Spacer({ width: 0, height: 0 });
  const totals = files.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 });
  function open(file) {
    return {
      type: "toolpkg.ui.open",
      placement: "workspace",
      routeId: ROUTE,
      instanceKey: "change-review",
      input: {
        chatId,
        messageTimestamp: message.timestamp,
        variantIndex: message.variantIndex,
        files: file === void 0 ? files : [file, ...files.filter((item) => item.path !== file.path)]
      }
    };
  }
  function capsule(value, added) {
    const color = added ? GREEN : RED;
    const alpha = { value: color, alpha: 0.12 };
    const border = { value: color, alpha: 0.28 };
    return UI.Box(
      { modifier: M.background(alpha, { cornerRadius: 4 }).border(0.8, border, { cornerRadius: 4 }).padding(5.8, 1.8) },
      UI.Text({ inheritStyle: true, text: `${added ? "+" : "-"}${value}`, color, fontWeight: "600", fontSize: 10.5, fontFamily: "monospace", lineHeight: 1.1 })
    );
  }
  function counts(file) {
    return UI.RichText({ spans: [
      { text: `+${file.additions}`, color: GREEN, fontWeight: "600", fontSize: 11, fontFamily: "monospace" },
      { text: "  " },
      { text: `-${file.deletions}`, color: RED, fontWeight: "600", fontSize: 11, fontFamily: "monospace" }
    ] });
  }
  function fileRow(file) {
    const { name, directory } = fileIdentity(file.path);
    return UI.InkWell(
      { shape: { cornerRadius: 6 }, onClick: () => open(file) },
      UI.Tooltip(
        { message: en ? `View diff for ${name}` : `查看 ${name} 的变更`, waitMillis: 400 },
        UI.Row({ modifier: M.padding(8, 5), verticalAlignment: "center" }, [
          UI.Svg({ source: fileIcon(name), width: 14, height: 14 }),
          UI.Spacer({ width: 8 }),
          UI.RichText({ weight: 1, maxLines: 1, overflow: "ellipsis", spans: [
            { text: name, fontSize: 12, fontWeight: "600", color: c.onSurface },
            ...directory.length === 0 ? [] : [
              { text: "  " },
              { text: directory, fontSize: 11, fontFamily: "monospace", color: c.onSurfaceVariant.copy({ alpha: 0.65 }) }
            ]
          ] }),
          UI.Spacer({ width: 8 }),
          counts(file),
          UI.Spacer({ width: 2 }),
          UI.Icon({ name: "chevron_right_rounded", size: 13, tint: c.onSurfaceVariant.copy({ alpha: 0.35 }) })
        ])
      )
    );
  }
  const header = UI.InkWell({
    shape: { topLeft: 10, topRight: 10, bottomLeft: expanded ? 0 : 10, bottomRight: expanded ? 0 : 10 },
    onClick: () => setExpanded(!expanded)
  }, UI.Row({ modifier: M.padding(10, 7), verticalAlignment: "center" }, [
    UI.Box(
      { modifier: M.background(c.primary.copy({ alpha: 0.12 }), { cornerRadius: 6 }).padding(4.5) },
      UI.Icon({ name: "difference_outlined", size: 14, tint: c.primary })
    ),
    UI.Spacer({ width: 8 }),
    UI.Text({ text: en ? `Edited ${files.length} ${files.length === 1 ? "file" : "files"}` : `已编辑 ${files.length} 个文件`, style: "labelMedium", color: c.onSurface, fontWeight: "600", letterSpacing: 0.1 }),
    UI.Spacer({ width: 8 }),
    UI.Row({ verticalAlignment: "center" }, [capsule(totals.additions, true), UI.Spacer({ width: 4 }), capsule(totals.deletions, false)]),
    UI.Box({ weight: 1 }),
    UI.TextButton({
      compact: true,
      contentColor: c.onSurfaceVariant.copy({ alpha: 0.8 }),
      contentPadding: { horizontal: 8, vertical: 4 },
      onClick: () => ({ type: "toolpkg.message.rollback" })
    }, [
      UI.Icon({ name: "undo_rounded", size: 14 }),
      UI.Spacer({ width: 8 }),
      UI.Text({ inheritStyle: true, text: en ? "Undo" : "撤销", fontSize: 11.5 })
    ]),
    UI.Spacer({ width: 4 }),
    UI.InkWell(
      { shape: { cornerRadius: 16 }, onClick: () => open() },
      UI.Box(
        { modifier: M.background(c.primary.copy({ alpha: 0.12 }), { cornerRadius: 16 }).border(0.8, c.primary.copy({ alpha: 0.35 }), { cornerRadius: 16 }).padding(9.8, 4.3) },
        UI.Row({ verticalAlignment: "center" }, [
          UI.Text({ inheritStyle: true, text: en ? "View changes" : "查看变更", fontSize: 11, fontWeight: "500", color: c.primary }),
          UI.Spacer({ width: 2 }),
          UI.Icon({ name: "arrow_forward_rounded", size: 11, tint: c.primary })
        ])
      )
    ),
    UI.Spacer({ width: 4 }),
    UI.Box({ modifier: M.padding(2) }, UI.Icon({ name: "keyboard_arrow_down", size: 16, tint: c.onSurfaceVariant.copy({ alpha: 0.75 }), turns: expanded ? 0.5 : 0, durationMillis: 200 }))
  ]));
  const body = UI.Box(
    { animation: { animateContentSize: true, durationMillis: 220, curve: "easeOutCubic", alignment: "topCenter" } },
    expanded ? UI.Column({ fillMaxWidth: true }, [
      UI.Divider({ height: 1, thickness: 0.8, color: c.outline.copy({ alpha: 0.15 }) }),
      UI.Box(
        { modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.18 })).heightIn(0, 240) },
        UI.ScrollArea(
          { thumbVisibility: files.length > 5 },
          UI.Column({ modifier: M.padding(4), fillMaxWidth: true }, files.map(fileRow))
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
        containerColor: c.surfaceContainerLowest,
        shape: { cornerRadius: 10 },
        border: { color: c.outline, alpha: 0.25, width: 0.8 }
      },
      UI.Column({ fillMaxWidth: true }, [header, body])
    )
  );
}
