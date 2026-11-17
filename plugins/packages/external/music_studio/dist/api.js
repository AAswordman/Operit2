"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.request = request;
function request(payload) { return ToolPkg.callDependency("com.operit.music_studio", "request", payload); }
