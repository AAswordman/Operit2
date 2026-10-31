import type {
  ComposeDslContext,
  ComposeNode,
} from "../../../../../types/compose-dsl";
import qrcode from "../vendor/qrcode.js";
import { call } from "../rpc.js";
import {
  errorText,
  type AutoReplyParams,
  type ReplyRole,
  type Status,
} from "../model.js";

/** Renders native QR onboarding, role selection, lifecycle controls and delivery diagnostics. */
export default function settings(ctx: ComposeDslContext): ComposeNode {
  const initialized = ctx.useRef("initialized", false);
  const working = ctx.useRef("working", false);
  const [status, setStatus] = ctx.useState<Status | null>("status", null);
  const [busy, setBusy] = ctx.useState("busy", "");
  const [error, setError] = ctx.useState("error", "");
  const [message, setMessage] = ctx.useState("message", "");
  const [roles, setRoles] = ctx.useState<ReplyRole[]>("roles", []);
  const [roleError, setRoleError] = ctx.useState("roleError", "");
  const [selection, setSelection] = ctx.useState("selection", "");
  const [enabled, setEnabled] = ctx.useState("enabled", true);
  const [instruction, setInstruction] = ctx.useState("instruction", "");
  const [group, setGroup] = ctx.useState("group", "微信 Bot");
  const [profile, setProfile] = ctx.useState("profile", "default");
  const [deadline, setDeadline] = ctx.useState("deadline", "120");
  const [verifyCode, setVerifyCode] = ctx.useState("verifyCode", "");
  const [advanced, setAdvanced] = ctx.useState("advanced", false);
  const [diagnostics, setDiagnostics] = ctx.useState("diagnostics", false);
  const [events, setEvents] = ctx.useState<string>("events", "");
  const [confirmLogout, setConfirmLogout] = ctx.useState(
    "confirmLogout",
    false,
  );
  const colors = ctx.MaterialTheme.colorScheme;

  /** Reads live state without replacing any unsaved form fields. */
  async function refresh(): Promise<void> {
    const result = await call({ action: "status" });
    if (!("status" in result)) throw new Error("微信状态返回格式错误。");
    setStatus(result.status);
  }

  /** Loads role choices independently so a dependency error does not hide login controls. */
  async function loadRoles(): Promise<void> {
    try {
      const result = await call({ action: "reply_roles" });
      if ("roles" in result) setRoles(result.roles);
      setRoleError("");
    } catch (failure) {
      setRoleError(errorText(failure));
    }
  }

  /** Initializes saved reply drafts once while keeping failed loads retryable. */
  async function load(): Promise<void> {
    if (initialized.current || working.current) return;
    working.current = true;
    try {
      const result = await call({ action: "status" });
      if (!("status" in result)) throw new Error("微信状态返回格式错误。");
      const config = result.status.autoReply.config;
      setStatus(result.status);
      setSelection(config.characterSelection);
      setEnabled(config.enabled);
      setInstruction(config.assistantInstruction);
      setGroup(config.chatGroup);
      setProfile(config.waifuProfile);
      setDeadline(String(config.aiTimeoutMs / 1000));
      initialized.current = true;
      setError("");
    } catch (failure) {
      setError(errorText(failure));
    } finally {
      working.current = false;
    }
    await loadRoles();
  }

  /** Serializes visible actions and reports real errors without overwriting drafts. */
  async function operate(
    label: string,
    operation: () => Promise<string>,
  ): Promise<void> {
    if (working.current) return;
    working.current = true;
    setBusy(label);
    setError("");
    setMessage("");
    try {
      setMessage(await operation());
    } catch (failure) {
      setError(errorText(failure));
    } finally {
      try {
        await refresh();
      } catch (failure) {
        setError(errorText(failure));
      }
      working.current = false;
      setBusy("");
    }
  }

  /** Validates user-facing seconds and returns the exact reply settings patch. */
  function replies(): AutoReplyParams {
    if (
      !/^\d+(?:\.\d{1,3})?$/.test(deadline) ||
      Number(deadline) < 1 ||
      Number(deadline) > 600
    ) {
      throw new Error("回复等待时间请填写 1～600 秒。");
    }
    if (profile.trim() === "") throw new Error("请填写 Waifu 配置名称。");
    return {
      enabled,
      character_selection: selection,
      assistant_instruction: instruction,
      chat_group: group.trim(),
      waifu_profile: profile.trim(),
      ai_timeout_ms: Math.round(Number(deadline) * 1000),
    };
  }

  /** Draws supporting text with the active host theme. */
  function description(value: string): ComposeNode {
    return ctx.UI.Text({
      text: value,
      style: "bodySmall",
      color: colors.onSurfaceVariant,
    });
  }

  /** Creates an action that cannot fire twice during one pending operation. */
  function button(
    key: string,
    label: string,
    onClick: () => void | Promise<void>,
    primary = false,
  ): ComposeNode {
    return ctx.UI[primary ? "Button" : "TextButton"]({
      key,
      text: label,
      onClick,
      enabled: busy === "" && status !== null,
      fillMaxWidth: primary,
    });
  }

  /** Builds one stable editable field without coupling it to status refreshes. */
  function field(
    key: string,
    label: string,
    value: string,
    update: (value: string) => void,
    multiline = false,
  ): ComposeNode {
    return ctx.UI.OutlinedTextField({
      key,
      label,
      value,
      onValueChange: update,
      fillMaxWidth: true,
      enabled: busy === "",
      singleLine: !multiline,
      minLines: multiline ? 3 : 1,
      maxLines: multiline ? 6 : 1,
    });
  }

  /** Groups controls into a native Material surface. */
  function card(
    key: string,
    title: string,
    children: ComposeNode[],
  ): ComposeNode {
    return ctx.UI.Card(
      {
        key,
        fillMaxWidth: true,
        containerColor: colors.surfaceContainerLow,
        shape: { cornerRadius: 16 },
      },
      ctx.UI.Column({ fillMaxWidth: true, padding: 16, spacing: 12 }, [
        ctx.UI.Text({ text: title, style: "titleMedium" }),
        ...children,
      ]),
    );
  }

  ctx.useEffect(
    "statusPoll",
    () => {
      let disposed = false;
      let timer: number | null = null;
      /** Observes QR progress and runtime state only while this settings screen exists. */
      async function tick(): Promise<void> {
        try {
          if (!working.current && initialized.current) {
            const result = await call({ action: "status" });
            if (!disposed && "status" in result) setStatus(result.status);
          }
        } catch (failure) {
          if (!disposed) setError(errorText(failure));
        } finally {
          if (!disposed)
            timer = setTimeout(() => {
              void tick();
            }, 1500);
        }
      }
      timer = setTimeout(() => {
        void tick();
      }, 1500);
      return () => {
        disposed = true;
        if (timer !== null) clearTimeout(timer);
      };
    },
    [],
  );

  const labels = {
    stopped: "机器人已停止",
    connecting: "正在连接微信…",
    ready: "微信已连接",
    error: "微信连接失败",
  };
  const state =
    status === null ? "正在读取配置…" : labels[status.service.state];
  const choices = [
    { selection: "", name: "跟随当前角色" },
    { selection: "model:chat", name: "模型对话" },
    ...roles,
  ];
  if (!choices.some((role) => role.selection === selection))
    choices.push({ selection, name: "已保存的角色" });
  const qrSvg = ctx.useMemo(
    "qr",
    () => {
      if (!status?.login.qrContent) return "";
      const code = qrcode(0, "M");
      code.addData(status.login.qrContent, "Byte");
      code.make();
      return code.createSvgTag({ cellSize: 4, margin: 16, scalable: true });
    },
    [status?.login.qrContent],
  );

  const connection: ComposeNode[] = [
    description(
      status?.account
        ? `已登录 Bot：${status.account.botId}`
        : "点击获取二维码，用微信扫码并确认。",
    ),
    button(
      "login",
      status?.account ? "重新扫码登录" : "获取登录二维码",
      () =>
        operate("正在获取二维码…", async () => {
          await call({ action: "login_start" });
          return "请用微信扫描二维码并确认。";
        }),
      true,
    ),
  ];
  if (qrSvg !== "")
    connection.push(
      ctx.UI.Svg({ key: "qrCode", source: qrSvg, width: 260, height: 260 }),
      description(
        status?.login.state === "scaned"
          ? "已扫码，请在微信中确认。"
          : "等待扫码；二维码有效期约五分钟。",
      ),
      field(
        "qrLink",
        "登录二维码内容（可复制到另一台设备）",
        status!.login.qrContent,
        () => {},
      ),
      button("cancelLogin", "取消本次登录", () =>
        operate("正在取消登录…", async () => {
          await call({ action: "login_cancel" });
          return "已取消扫码登录。";
        }),
      ),
    );
  if (status?.login.state === "need_verifycode")
    connection.push(
      field("verifyCode", "微信显示的验证码", verifyCode, setVerifyCode),
      button(
        "verify",
        "提交验证码",
        () =>
          operate("正在验证…", async () => {
            await call({ action: "login_verify", code: verifyCode });
            setVerifyCode("");
            return "验证码已提交。";
          }),
        true,
      ),
    );
  if (status?.login.error) connection.push(description(status.login.error));
  if (status?.account)
    connection.push(button("logout", "退出登录", () => setConfirmLogout(true)));
  if (confirmLogout)
    connection.push(
      description(
        "退出后会删除本设备登录凭据和待处理消息，已有原生聊天记录保留。",
      ),
      button("confirmLogout", "确认退出登录", () =>
        operate("正在退出登录…", async () => {
          await call({ action: "logout" });
          setConfirmLogout(false);
          return "已退出微信登录。";
        }),
      ),
      button("keepAccount", "取消", () => setConfirmLogout(false)),
    );

  const reply: ComposeNode[] = [
    ctx.UI.DropdownMenu(
      {
        key: "role",
        label: "回复角色",
        text: choices.find((role) => role.selection === selection)!.name,
        enabled: busy === "",
        fillMaxWidth: true,
        onClick: (index) => setSelection(choices[index].selection),
      },
      choices.map((role) =>
        ctx.UI.Text({ text: role.name, paddingVertical: 8 }),
      ),
    ),
    description(
      "每位微信用户都有独立的原生聊天。角色变更用于新会话，旧会话保留原角色和历史。",
    ),
    ctx.UI.Row(
      { fillMaxWidth: true, verticalAlignment: "center", spacing: 12 },
      [
        ctx.UI.Text({ text: "AI 自动回复", weight: 1 }),
        ctx.UI.Switch({
          key: "autoReply",
          checked: enabled,
          onCheckedChange: setEnabled,
          enabled: busy === "",
        }),
      ],
    ),
    ...(roleError
      ? [
          description(`角色列表读取失败：${roleError}`),
          button("reloadRoles", "重新读取角色", loadRoles),
        ]
      : []),
    button("advanced", advanced ? "收起高级设置" : "高级设置", () =>
      setAdvanced(!advanced),
    ),
  ];
  if (advanced)
    reply.push(
      field("deadline", "回复等待时间（秒）", deadline, setDeadline),
      field("instruction", "回复要求", instruction, setInstruction, true),
      field("chatGroup", "聊天记录分组", group, setGroup),
      field("waifuProfile", "Waifu 配置名称", profile, setProfile),
      description("分句、打字节奏、情绪表情和自拍效果在 Waifu 插件中配置。"),
    );
  reply.push(
    button("saveReplies", "保存回复设置", () =>
      operate("正在保存设置…", async () => {
        await call({ action: "auto_configure", params: replies() });
        return "回复设置已保存。";
      }),
    ),
  );
  if (status?.listenerEnabled && status.service.state !== "error")
    reply.push(
      button(
        "stop",
        "停止机器人",
        () =>
          operate("正在停止机器人…", async () => {
            await call({ action: "stop" });
            return "机器人已停止，下次启动应用也会保持停止。";
          }),
        true,
      ),
    );
  else
    reply.push(
      button(
        "start",
        "保存并启动机器人",
        () =>
          operate("正在启动机器人…", async () => {
            await call({ action: "auto_configure", params: replies() });
            await call({ action: "start" });
            return "机器人已启动，可以在微信中发消息。";
          }),
        true,
      ),
    );

  const children: ComposeNode[] = [
    ctx.UI.Text({ text: "微信机器人", style: "headlineMedium" }),
    description("扫码连接，选择角色，在微信中与你的 Operit 对话。"),
    card("statusCard", state, [
      description(
        `待处理 ${status?.queue.size ?? 0} 条 · 已回复 ${status?.autoReply.repliedCount ?? 0} 条`,
      ),
      ...(busy ? [description(busy)] : []),
      ...(status?.service.error
        ? [ctx.UI.Text({ text: status.service.error, color: colors.error })]
        : []),
      ...(error ? [ctx.UI.Text({ text: error, color: colors.error })] : []),
      ...(message ? [description(message)] : []),
      ctx.UI.TextButton({
        key: "refresh",
        text: "刷新状态",
        enabled: busy === "",
        onClick: async () => {
          if (!initialized.current) await load();
          else
            await operate("正在刷新…", async () => {
              await refresh();
              return "";
            });
        },
      }),
    ]),
    card("connectionCard", "接入微信", connection),
    card("replyCard", "回复设置", reply),
  ];
  const delivery = status?.autoReply.delivery;
  if (delivery !== null && delivery !== undefined)
    children.push(
      card("deliveryCard", "需要处理的回复", [
        description(
          `消息 ${delivery.messageId} · 已发送 ${delivery.delivered} 段`,
        ),
        ctx.UI.Text({
          text: delivery.error || "正在回复…",
          color: colors.error,
        }),
        ...(delivery.state === "failed"
          ? [
              description(
                "后续自动回复已暂停。重试整个请求可能重复已发送的内容，请先在微信中核对。",
              ),
              button("retryDelivery", "重试整个请求", () =>
                operate("正在安排重试…", async () => {
                  await call({
                    action: "delivery_resolve",
                    resolution: "retry",
                  });
                  return "已安排重试。";
                }),
              ),
              button("discardDelivery", "跳过这条回复，继续后续消息", () =>
                operate("正在跳过…", async () => {
                  await call({
                    action: "delivery_resolve",
                    resolution: "discard",
                  });
                  return "已跳过失败回复。";
                }),
              ),
            ]
          : []),
      ]),
    );
  children.push(
    button("diagnostics", diagnostics ? "收起诊断信息" : "诊断信息", () =>
      setDiagnostics(!diagnostics),
    ),
  );
  if (diagnostics)
    children.push(
      card("diagnosticsCard", "诊断信息", [
        description(
          `已接收 ${status?.service.receivedCount ?? 0} 条 · 会话绑定 ${status?.autoReply.bindingCount ?? 0} 个`,
        ),
        description(
          "应用需保持运行。登录凭据只保存在本设备，不随空间同步。停止和退出会等待当前网络请求结束。",
        ),
        button("peekEvents", "查看待处理消息", () =>
          operate("正在读取消息…", async () => {
            const result = await call({
              action: "receive",
              params: { limit: 10, consume: false },
            });
            if ("events" in result)
              setEvents(
                result.events
                  .map(
                    (event) => `${event.userId}: ${event.text || "[媒体消息]"}`,
                  )
                  .join("\n"),
              );
            return "";
          }),
        ),
        description(events || "暂无消息预览。"),
        button("clearEvents", "清空待处理消息", () =>
          operate("正在清空消息…", async () => {
            await call({ action: "clear" });
            setEvents("");
            return "待处理消息已清空。";
          }),
        ),
      ]),
    );
  return ctx.UI.LazyColumn(
    { fillMaxSize: true, padding: 16, spacing: 16, onLoad: load },
    children,
  );
}
