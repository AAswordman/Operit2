"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.call = call;
const model_js_1 = require("./model.js");
/** Routes tools and Compose actions to the single package main runtime. */
function call(request) {
    return ToolPkg.ipc.call(model_js_1.CHANNEL, request, {
        targetRuntime: "main",
    });
}
