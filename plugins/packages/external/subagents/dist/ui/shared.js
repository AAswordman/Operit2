"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.statusIcons = exports.labels = exports.CONVERSATION_ROUTE = void 0;
exports.CONVERSATION_ROUTE = "toolpkg:com.operit.subagents:ui:conversation";
exports.labels = {
    starting: "正在启动", running: "运行中", completed: "已完成", cancelled: "已取消",
    failed: "执行失败", closing: "正在关闭", closed: "已关闭", interrupted: "观察已中断",
};
exports.statusIcons = {
    starting: "hourglassEmpty", running: "playArrow", completed: "checkCircle",
    cancelled: "cancel", failed: "errorOutline", closing: "hourglassEmpty",
    closed: "close", interrupted: "pause",
};
