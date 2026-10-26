import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import { call } from "../rpc.js";
import type { ConfigureParams, Request, Status } from "../model.js";
import { errorText } from "../model.js";

/** Renders QQ credentials, native listener state and automatic reply controls. */
export default function settings(ctx: ComposeDslContext): ComposeNode {
  const [initialized, setInitialized] = ctx.useState("initialized", false);
  const [busy, setBusy] = ctx.useState("busy", false);
  const [status, setStatus] = ctx.useState<Status | null>("status", null);
  const [error, setError] = ctx.useState("error", "");
  const [message, setMessage] = ctx.useState("message", "");
  const [appId, setAppId] = ctx.useState("appId", "");
  const [secret, setSecret] = ctx.useState("secret", "");
  const [sandbox, setSandbox] = ctx.useState("sandbox", false);
  const [listener, setListener] = ctx.useState("listener", false);
  const [autoEnabled, setAutoEnabled] = ctx.useState("autoEnabled", false);
  const [c2c, setC2c] = ctx.useState("c2c", true);
  const [group, setGroup] = ctx.useState("group", true);
  const [selection, setSelection] = ctx.useState("selection", "");
  const [instruction, setInstruction] = ctx.useState("instruction", "");
  const [chatGroup, setChatGroup] = ctx.useState("chatGroup", "QQ Bot");
  const [waifuProfile, setWaifuProfile] = ctx.useState("waifuProfile", "default");
  const [deadline, setDeadline] = ctx.useState("deadline", "120000");

  /** Reflects persisted settings without ever reading the saved secret into the UI. */
  function sync(value: Status): void {
    setWaifuProfile(value.waifuProfile); setStatus(value); setAppId(value.appId); setSandbox(value.useSandbox); setListener(value.listenerEnabled);
    setAutoEnabled(value.autoReply.config.enabled); setC2c(value.autoReply.config.c2cEnabled); setGroup(value.autoReply.config.groupEnabled);
    setSelection(value.autoReply.config.characterSelection); setInstruction(value.autoReply.config.assistantInstruction);
    setChatGroup(value.autoReply.config.chatGroup); setDeadline(String(value.autoReply.config.aiTimeoutMs));
  }

  /** Runs one main-runtime operation and presents its original outcome. */
  async function run(request: Request, refreshForm = false): Promise<void> {
    if (busy) return;
    setBusy(true); setError(""); setMessage("");
    try {
      const result = await call(request);
      if (request.action === "configure") setSecret("");
      if ("status" in result) { if (refreshForm) sync(result.status); else setStatus(result.status); }
      else setMessage(JSON.stringify(result, null, 2));
    } catch (failure) { console.error("[qqbot.ui]", failure); setError(errorText(failure)); }
    finally { setBusy(false); }
  }

  /** Builds an editable field with the common native Compose appearance. */
  function field(label: string, value: string, update: (value: string) => void, sensitive = false): ComposeNode {
    return ctx.UI.TextField({ label, value, onValueChange: update, enabled: !busy, singleLine: true, isPassword: sensitive, fillMaxWidth: true });
  }

  /** Builds a native switch with a visible setting label. */
  function toggle(label: string, checked: boolean, update: (value: boolean) => void): ComposeNode {
    return ctx.UI.Row({ fillMaxWidth: true, horizontalArrangement: "spaceBetween", verticalAlignment: "center" }, [ctx.UI.Text({ text: label }), ctx.UI.Switch({ checked, onCheckedChange: update, enabled: !busy })]);
  }

  /** Builds a command button whose operation is owned by the main service. */
  function button(label: string, request: Request): ComposeNode {
    return ctx.UI.Button({ text: label, enabled: !busy, onClick: () => run(request, true) });
  }

  /** Saves only an explicitly entered replacement secret. */
  async function save(): Promise<void> {
    const params: ConfigureParams = { app_id: appId, use_sandbox: sandbox, listener_enabled: listener };
    if (secret !== "") params.app_secret = secret;
    await run({ action: "configure", params }, true);
  }

  /** Saves AI conversation settings through the same authoritative command path. */
  async function saveAuto(): Promise<void> {
    await run({ action: "auto_configure", params: { waifu_profile: waifuProfile, enabled: autoEnabled, c2c_enabled: c2c, group_enabled: group, character_selection: selection, chat_group: chatGroup, assistant_instruction: instruction, ai_timeout_ms: Number(deadline), start_now: true } }, true);
  }

  const children: ComposeNode[] = [
    ctx.UI.Text({ text: "QQ Bot", style: "headlineMedium" }),
    ctx.UI.Text({ text: "使用腾讯 QQ Bot 的 AppID 和 AppSecret。机器人事件中提供的 openid 用于收发消息。", style: "bodyMedium" }),
    ctx.UI.Card({ fillMaxWidth: true }, [ctx.UI.Column({ padding: 16, spacing: 12 }, [
      ctx.UI.Text({ text: "应用配置", style: "titleMedium" }),
      field("AppID", appId, setAppId), field("AppSecret（输入以更新）", secret, setSecret, true),
      toggle("沙箱环境", sandbox, setSandbox), toggle("应用启动时监听", listener, setListener),
      ctx.UI.Button({ text: "保存配置", enabled: !busy, onClick: save }), button("验证连接", { action: "test" }),
    ])]),
    ctx.UI.Card({ fillMaxWidth: true }, [ctx.UI.Column({ padding: 16, spacing: 12 }, [
      ctx.UI.Text({ text: "连接与事件", style: "titleMedium" }),
      ctx.UI.Text({ text: status === null ? "正在读取状态" : `Gateway: ${status.service.status}\n队列: ${status.queue.size}/${status.queue.capacity}\n收到事件: ${status.service.receivedCount}` }),
      button("启动监听", { action: "start" }), button("重启监听", { action: "start", restart: true }), button("停止监听", { action: "stop" }),
      button("刷新状态", { action: "status" }), button("查看最近事件", { action: "receive", params: { limit: 10, consume: false } }), button("清空事件队列", { action: "clear" }),
    ])]),
    ctx.UI.Card({ fillMaxWidth: true }, [ctx.UI.Column({ padding: 16, spacing: 12 }, [
      ctx.UI.Text({ text: "AI 自动回复", style: "titleMedium" }),
      toggle("启用自动回复", autoEnabled, setAutoEnabled), toggle("回复私聊", c2c, setC2c), toggle("回复群消息", group, setGroup),
      field("Waifu 配置名称", waifuProfile, setWaifuProfile),
      ctx.UI.Text({ text: "逐句回复和表情由独立 Waifu 模式插件提供。请到 Waifu 模式设置页修改对应配置。", style: "bodySmall" }),
      field("会话分组", chatGroup, setChatGroup), field("角色选择标识（新会话生效）", selection, setSelection),
      field("AI 超时（毫秒）", deadline, setDeadline), field("回复指令", instruction, setInstruction),
      ctx.UI.Text({ text: "留空角色选择标识会使用当前角色配置。每个私聊或群绑定独立会话，绑定在重新启动后保留。", style: "bodySmall" }),
      ctx.UI.Button({ text: "保存自动回复设置", enabled: !busy, onClick: saveAuto }),
      button("启动自动回复", { action: "auto_start" }), button("停止自动回复", { action: "auto_stop" }),
      ctx.UI.Text({ text: status === null ? "" : `自动回复: ${status.autoReply.running ? "运行中" : "已停止"}\n已回复: ${status.autoReply.repliedCount}` }),
    ])]),
  ];
  if (status !== null && status.service.lastError !== "") children.push(ctx.UI.Text({ text: status.service.lastError, color: "error" }));
  if (status !== null && status.autoReply.lastError !== "") children.push(ctx.UI.Text({ text: status.autoReply.lastError, color: "error" }));
  if (error !== "") children.push(ctx.UI.Text({ text: error, color: "error" }));
  if (message !== "") children.push(ctx.UI.Text({ text: message, style: "bodySmall" }));
  return ctx.UI.LazyColumn({ fillMaxSize: true, padding: 16, spacing: 16, onLoad: async () => {
    if (initialized) return;
    setInitialized(true);
    await run({ action: "status" }, true);
  } }, children);
}
