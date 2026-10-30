"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
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
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/main.ts
var main_exports = {};
__export(main_exports, {
  HOME_ROUTE: () => HOME_ROUTE,
  SUMMARY_ROUTE: () => SUMMARY_ROUTE,
  WORKSPACE_ROUTE: () => WORKSPACE_ROUTE,
  registerToolPkg: () => registerToolPkg
});
module.exports = __toCommonJS(main_exports);
var import_summary_ui = __toESM(require("./ui/summary.ui.js"));
var import_workspace_ui = __toESM(require("./ui/workspace.ui.js"));
var import_home_ui = __toESM(require("./ui/home.ui.js"));
var SUMMARY_ROUTE = "toolpkg:com.operit.change_review:ui:summary";
var WORKSPACE_ROUTE = "toolpkg:com.operit.change_review:ui:workspace";
var HOME_ROUTE = "toolpkg:com.operit.change_review:ui:home";
function registerToolPkg() {
  ToolPkg.registerUiRoute({ id: "home", route: HOME_ROUTE, runtime: "compose_dsl", screen: import_home_ui.default, title: { zh: "代码变更审查", en: "Code change review" } });
  ToolPkg.registerUiRoute({ id: "summary", route: SUMMARY_ROUTE, runtime: "compose_dsl", screen: import_summary_ui.default, title: { zh: "文件变更", en: "File changes" } });
  ToolPkg.registerUiRoute({ id: "workspace", route: WORKSPACE_ROUTE, runtime: "compose_dsl", screen: import_workspace_ui.default, keepAlive: true, title: { zh: "变更", en: "Changes" } });
  ToolPkg.registerNavigationEntry({ id: "message_changes", route: SUMMARY_ROUTE, surface: "chat_message_after", title: { zh: "文件变更", en: "File changes" }, order: 100 });
  ToolPkg.registerNavigationEntry({ id: "workspace_changes", route: WORKSPACE_ROUTE, surface: "workspace", title: { zh: "变更", en: "Changes" }, icon: "difference_outlined", order: 100 });
  ToolPkg.registerNavigationEntry({ id: "home_changes", route: HOME_ROUTE, surface: "workspace_home", title: { zh: "代码变更审查", en: "Code change review" }, order: 100 });
  return true;
}
