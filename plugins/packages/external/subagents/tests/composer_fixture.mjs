import { harness, modules } from "./runtime.mjs";
import { fileURLToPath } from "node:url";

/** Exports the authored active composer for geometry checks in the real Flutter DSL renderer. */
async function composerFixture() {
  const h = harness();
  const child = await h.spawn({ name: "检查子代理区域在窄屏和大字体下的对齐与长名称显示" });
  const parent = await h.manager.request({ action: "list", parentChatId: "parent" });
  const state = new Map([
    ["chatId", "parent"],
    ["subagents.parent", parent],
    ["subagents.loaded", true],
  ]);
  const screen = modules().load("./ui/panel.ui").default;
  const ctx = {
    UI: new Proxy({}, {
      /** Preserves authored nodes and all layout props across the JSON boundary. */
      get(_target, type) {
        return (props = {}, children = []) => {
          const { content, ...attributes } = props;
          return {
            type,
            props: attributes,
            children: Array.isArray(children) ? children : [children],
            slots: content === undefined ? {} : { content: Array.isArray(content) ? content : [content] },
          };
        };
      },
    }),
    MaterialTheme: { colorScheme: { error: "#ff0000" } },
    /** Supplies the native manager snapshot without mounting host subscriptions. */
    useState(key, initial) {
      if (!state.has(key)) state.set(key, initial);
      return [state.get(key), value => state.set(key, value)];
    },
    /** Supplies the selected-task reference for the authored screen. */
    useRef(_key, initial) { return { current: initial }; },
    /** Leaves observation to the manager fixture while exporting a single snapshot. */
    useEffect() {},
    /** Rejects unexpected presentation failures. */
    reportError(error) { throw error; },
  };
  const node = screen(ctx);
  state.set("subagents.expanded", true);
  const expandedNode = screen(ctx);
  const goalScreen = modules({ ToolPkg: {
    /** Keeps route decorators inert while geometry snapshots own no host execution. */
    route() {
      /** Retains the authored routed method without invoking it. */
      return value => value;
    },
  } }).load(fileURLToPath(new URL("../../../buildin/goal_mode/src/ui/goal_panel/index.ui", import.meta.url))).default;
  state.set("goal", { objective: "检查目标模式在窄屏和大字体下的对齐与长目标显示", status: "active", startedAt: 1000, pausedDurationMs: 0 });
  const goalNode = goalScreen(ctx);
  state.set("goal.expanded", true);
  return { agentId: child.id, node, expandedNode, goalNode, goalExpandedNode: goalScreen(ctx) };
}

const fixture = await composerFixture();
/** Encodes callbacks as host action identities so Flutter renders interactive controls normally. */
process.stdout.write(JSON.stringify(fixture, (_key, value) => typeof value === "function" ? "fixture.action" : value));
