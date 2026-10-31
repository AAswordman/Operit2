import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";

const CONTROL_ROUTE = "toolpkg:com.operit.floating_window_demo:ui:control";
const PET_ROUTE = "toolpkg:com.operit.floating_window_demo:ui:pet";

/** A live, transparent route; its state survives hide/show and window updates. */
export function pet(ctx: ComposeDslContext): ComposeNode {
  const [count, setCount] = ctx.useState("count", 0);
  const position = ctx.useRef("position", 100);
  return ctx.UI.Column({ spacing: 8, padding: 8 }, [
    ctx.UI.Text({ text: count % 2 ? "😸" : "🐱", fontSize: 64 }),
    ctx.UI.Button({ text: `摸摸 ${count}`, onClick: () => setCount(count + 1) }),
    ctx.UI.Button({ text: "向右移动", onClick: async () => {
      position.current += 20;
      await ToolPkg.ui.floatingWindows.update("pet", { x: position.current });
    } }),
    ctx.UI.Button({ text: "关闭", onClick: async () => { await ToolPkg.ui.floatingWindows.close("pet"); } }),
  ]);
}

/** Explicit permission control and basic lifecycle examples for every host. */
export function control(ctx: ComposeDslContext): ComposeNode {
  const [status, setStatus] = ctx.useState("status", "先查询当前平台能力");
  function run(action: () => Promise<unknown>): () => Promise<void> {
    return async () => {
      try { setStatus(JSON.stringify(await action()) ?? "完成"); }
      catch (error) { setStatus(String(error)); }
    };
  }
  return ctx.UI.Column({ padding: 16, spacing: 12 }, [
    ctx.UI.Text({ text: "通用悬浮窗示例", style: "headlineSmall" }),
    ctx.UI.Text({ text: status }),
    ctx.UI.Button({ text: "查询能力", onClick: run(() => ToolPkg.ui.floatingWindows.capabilities()) }),
    ctx.UI.Button({ text: "申请悬浮窗权限", onClick: run(() => ToolPkg.ui.floatingWindows.requestPermission()) }),
    ctx.UI.Button({ text: "创建桌宠", onClick: run(() => ToolPkg.ui.floatingWindows.open({
      id: "pet", route: PET_ROUTE, options: { x: 100, y: 100, width: 200, height: 280 },
    })) }),
    ctx.UI.Button({ text: "隐藏", onClick: run(() => ToolPkg.ui.floatingWindows.hide("pet")) }),
    ctx.UI.Button({ text: "显示", onClick: run(() => ToolPkg.ui.floatingWindows.show("pet")) }),
    ctx.UI.Button({ text: "关闭", onClick: run(() => ToolPkg.ui.floatingWindows.close("pet")) }),
  ]);
}

export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "control", route: CONTROL_ROUTE, runtime: "compose_dsl", screen: control,
    title: { zh: "悬浮窗示例", en: "Floating Window Demo" } });
  ToolPkg.registerUiRoute({ id: "pet", route: PET_ROUTE, runtime: "compose_dsl", screen: pet,
    title: { zh: "桌宠", en: "Pet" } });
  ToolPkg.registerNavigationEntry({ id: "control", route: CONTROL_ROUTE, surface: "toolbox",
    title: { zh: "悬浮窗示例", en: "Floating Window Demo" }, icon: "PictureInPicture" });
  return true;
}
