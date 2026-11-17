"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.call = call;
exports.batch = batch;
exports.json = json;
const model_1 = require("../shared/model");
function call(request) { return ToolPkg.ipc.call("music.service", request, { targetRuntime: "main" }); }
function batch(p, operations) { return call({ action: "batch", ...p, operations }); }
function json(value) { if (typeof value !== "string" || value.length > model_1.LIMITS.jsonBytes)
    throw new Error("Expected JSON string ≤8MB"); return JSON.parse(value); }
