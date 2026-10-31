import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { InitializationInspection, InitializationRecoveryRequest } from "./storage/initialization";

interface RecoveryUiState {
  failure: string;
  inspection: InitializationInspection | null;
  busy: boolean;
  confirmed: boolean;
}

/** Offers explicit recovery inside the management screen without requiring the broken domain service. */
export function initializationRecoveryUi(ctx: ComposeDslContext, onRecovered: () => void): {
  panel: ComposeNode | null;
  reportFailure: (failure: unknown) => void;
} {
  const [state, setState] = ctx.useState<RecoveryUiState>("initialization-recovery", { failure: "", inspection: null, busy: false, confirmed: false });
  const live = ctx.useRef("initialization-recovery-live", state); live.current = state;
  /** Updates the live action guard before scheduling a new Compose render. */
  function update(patch: Partial<RecoveryUiState>): void {
    live.current = { ...live.current, ...patch }; setState(live.current);
  }
  /** Displays the original failure instead of leaving recovery hidden behind a failed WebView bootstrap. */
  function reportFailure(failure: unknown): void {
    update({ failure: String(failure), inspection: null, confirmed: false });
  }
  /** Inspects actual records or executes only an expressly confirmed recovery request. */
  async function run(repair: boolean): Promise<void> {
    if (live.current.busy) return;
    const inspection = live.current.inspection;
    let request: InitializationRecoveryRequest = { action: "inspect" };
    if (repair) {
      if (!live.current.confirmed) throw new Error("请先确认修复说明。");
      if (inspection?.status === "repairable") request = { action: "repair", source: inspection.source };
      else if (inspection?.status === "planned") request = { action: "resume" };
      else throw new Error("请先检查初始化数据。");
    }
    update({ busy: true, confirmed: false });
    try {
      const result = await ToolPkg.ipc.call<InitializationRecoveryRequest, InitializationInspection>("character-memory.initialization", request, { targetRuntime: "main" });
      if (repair) {
        if (result.status !== "ready") throw new Error("初始化修复未完成。");
        update({ failure: "", inspection: null });
        onRecovered();
      } else update({ inspection: result });
    } catch (failure) { update({ failure: String(failure) }); }
    finally { update({ busy: false }); }
  }
  if (state.failure === "") return { panel: null, reportFailure };
  const inspection = state.inspection;
  const repairable = inspection?.status === "repairable" || inspection?.status === "planned";
  const lines: ComposeNode[] = [
    ctx.UI.Text({ text: "角色卡初始化未完成", style: "titleLarge" }),
    ctx.UI.Text({ text: state.failure, style: "bodyMedium" }),
    ctx.UI.TextButton({ key: "initialization-inspect", text: state.busy ? "正在处理…" : "检查初始化数据", enabled: !state.busy, onClick: () => run(false) }),
  ];
  if (inspection?.status === "ready") lines.push(ctx.UI.Text({ text: "初始化元数据已存在。当前错误不属于缺失首次初始化信息，不会修改现有数据。" }));
  if (inspection?.status === "repairable") lines.push(
    ctx.UI.Text({ text: "检测到旧版首次安装中断。将保留此角色的 ID 和全部内容，并将其设为当前角色：" }),
    ctx.UI.Text({ text: inspection.source.card.name, fontWeight: "700" }),
    ctx.UI.Text({ text: inspection.source.card.id, style: "bodySmall" }),
    ctx.UI.Text({ text: "仅补齐缺失的初始化信息、记忆空间和 USER.md。已有记忆设置与文件保持不变；缺失的全局记忆开关及召回设置使用初始值。不会删除或重置角色卡。" }),
  );
  if (inspection?.status === "planned") lines.push(ctx.UI.Text({ text: `已有“${inspection.cardName}”的初始化计划。将继续这份计划，保留原 ID 和已有文件，不重新安装。` }));
  if (repairable) lines.push(
    ctx.UI.Row({ spacing: 8, verticalAlignment: "center" }, [
      ctx.UI.Checkbox({ key: "initialization-confirm", checked: state.confirmed, enabled: !state.busy, onCheckedChange: checked => update({ confirmed: checked }) }),
      ctx.UI.Text({ text: "我已核对上述角色，确认按说明修复初始化", weight: 1 }),
    ]),
    ctx.UI.Button({ key: "initialization-repair", text: "修复初始化", enabled: state.confirmed && !state.busy, onClick: () => run(true) }),
  );
  return { panel: ctx.UI.Column({ key: "initialization-recovery-panel", fillMaxSize: true, padding: 20, spacing: 16 }, lines), reportFailure };
}
