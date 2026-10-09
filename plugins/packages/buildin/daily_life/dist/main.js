"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.onClock = onClock;
exports.onOpen = onOpen;
exports.onResume = onResume;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
const reminders_1 = require("./reminders");
// Tools, clock events and lifecycle events share this main-runtime storage owner.
ToolPkg.ipc.on("daily_life.reminders", reminders_1.receiveReminder);
/** Registers reminder scheduling through existing platform-neutral Host capabilities. */
function registerToolPkg() {
    ToolPkg.registerHostEventHook({
        id: "daily_life_reminder_clock", source: "interval",
        trigger: { kind: "interval", intervalMs: 60000 }, function: onClock,
    });
    ToolPkg.registerAppLifecycleHook({
        id: "daily_life_reminder_start", event: "application_on_create", function: onOpen,
    });
    ToolPkg.registerHostEventHook({
        id: "daily_life_reminder_resume", source: "broadcast",
        trigger: { kind: "broadcast", topic: "app.lifecycle.resumed" }, function: onResume,
    });
    return true;
}
/** Delivers due reminders when the Host emits the registered minute clock. */
async function onClock() {
    await (0, reminders_1.deliverReminders)();
}
/** Checks persisted overdue reminders after the application starts. */
async function onOpen() {
    await (0, reminders_1.deliverReminders)();
}
/** Checks persisted overdue reminders when the application resumes. */
async function onResume() {
    await (0, reminders_1.deliverReminders)();
}
/** A no-side-effect round trip through this plugin's actual Core main runtime. */
function test_connection() {
    return { passed: true };
}
/** Exercises the existing package tool dispatcher, not a fixture or direct date call. */
async function test_tool_call() {
    const result = await toolCall("daily_life:get_current_date", {});
    const value = typeof result === "string" ? JSON.parse(result) : result;
    return { passed: !!value && typeof value.iso === "string" && typeof value.timestamp === "number",
        message: "日期工具未返回有效结果" };
}
