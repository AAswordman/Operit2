"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.read = read;
exports.create = create;
exports.status = status;
function read(chatId) {
    return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.read", { chatId });
}
function create(chatId, objective) {
    return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.create", { chatId, objective });
}
function status(chatId, status) {
    return ToolPkg.callDependency("com.operit.goal_mode_bundle", "goal.status", { chatId, status });
}
