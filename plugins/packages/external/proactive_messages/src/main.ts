import screen from "./ui";
import { prepare, removeRule, saveRule, setForeground, snapshot, stop, tick } from "./service";

RuntimeContext.register({ prepare, removeRule, saveRule, snapshot, setForeground });
export function registerToolPkg(): boolean {
  ToolPkg.registerToolboxUiModule({ id: "proactive_messages_settings", runtime: "compose_dsl", screen,
    params: {}, title: { zh: "AI 主动消息", en: "AI Proactive Messages" } });
  ToolPkg.registerHostEventHook({ id: "proactive_messages_clock", source: "interval",
    trigger: { kind: "interval", intervalMs: 60000 }, function: onClock });
  ToolPkg.registerAppLifecycleHook({ id: "proactive_messages_start", event: "application_on_create", function: onClock });
  ToolPkg.registerAppLifecycleHook({ id: "proactive_messages_foreground", event: "application_on_foreground", function: onForeground });
  ToolPkg.registerAppLifecycleHook({ id: "proactive_messages_background", event: "application_on_background", function: onBackground });
  ToolPkg.registerAppLifecycleHook({ id: "proactive_messages_terminate", event: "application_on_terminate", function: onTerminate });
  for (const topic of ["app.lifecycle.resumed", "app.lifecycle.inactive", "app.lifecycle.paused", "app.lifecycle.hidden", "app.lifecycle.detached"] as const)
    ToolPkg.registerHostEventHook({ id: `proactive_${topic}`, source: "broadcast", trigger: { kind: "broadcast", topic }, function: onLifecycle });
  return true;
}
export async function onClock(): Promise<void> { await tick(); }
export async function onForeground(): Promise<void> { await setForeground(true); }
export async function onBackground(): Promise<void> { await setForeground(false); }
export function onTerminate(): void { stop(); }
export async function onLifecycle(event: ToolPkg.HostEventBroadcastHookEvent<"app.lifecycle.resumed" | "app.lifecycle.inactive" | "app.lifecycle.paused" | "app.lifecycle.hidden" | "app.lifecycle.detached">): Promise<void> {
  await setForeground(event.eventPayload.payload.topic === "app.lifecycle.resumed");
}
