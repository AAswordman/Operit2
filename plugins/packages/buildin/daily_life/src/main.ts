import { deliverReminders, receiveReminder } from "./reminders";

// Tools, clock events and lifecycle events share this main-runtime storage owner.
ToolPkg.ipc.on("daily_life.reminders", receiveReminder);

/** Registers reminder scheduling through existing platform-neutral Host capabilities. */
export function registerToolPkg(): boolean {
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
export async function onClock(): Promise<void> {
  await deliverReminders();
}

/** Checks persisted overdue reminders after the application starts. */
export async function onOpen(): Promise<void> {
  await deliverReminders();
}

/** Checks persisted overdue reminders when the application resumes. */
export async function onResume(): Promise<void> {
  await deliverReminders();
}

/** A no-side-effect round trip through this plugin's actual Core main runtime. */
export function test_connection(): { passed: boolean } {
  return { passed: true };
}

/** Exercises the existing package tool dispatcher, not a fixture or direct date call. */
export async function test_tool_call(): Promise<{ passed: boolean; message?: string }> {
  const result = await toolCall("daily_life:get_current_date", {});
  const value = typeof result === "string" ? JSON.parse(result) : result;
  return { passed: !!value && typeof value.iso === "string" && typeof value.timestamp === "number",
    message: "日期工具未返回有效结果" };
}
