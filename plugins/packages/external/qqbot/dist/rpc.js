"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.call = call;
const model_js_1 = require("./model.js");
/** Routes every tool and UI command to the package main runtime. */
function call(request) {
    return ToolPkg.ipc.call(model_js_1.CHANNEL, request, { targetRuntime: "main" });
}
