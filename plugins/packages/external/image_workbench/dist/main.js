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
  registerToolPkg: () => registerToolPkg
});
module.exports = __toCommonJS(main_exports);
var import_workbench_ui = __toESM(require("./ui/workbench.ui.js"));
var ROUTE = "toolpkg:com.operit.image_workbench:ui:workbench";
function registerToolPkg() {
  ToolPkg.registerUiRoute({ id: "workbench", route: ROUTE, runtime: "compose_dsl", screen: import_workbench_ui.default, keepAlive: true, title: { zh: "生图工作台", en: "Image Workbench" } });
  ToolPkg.registerNavigationEntry({ id: "workbench", route: ROUTE, surface: "main_sidebar_plugins", title: { zh: "生图工作台", en: "Image Workbench" }, icon: "palette", order: 40 });
  ToolPkg.registerNavigationEntry({ id: "workbench_toolbox", route: ROUTE, surface: "toolbox", title: { zh: "生图工作台", en: "Image Workbench" }, icon: "palette", order: 40 });
  return true;
}
