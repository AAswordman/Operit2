import type { ComposeDslContext, ComposeNode, ComposePickedFile } from "../../../../types/compose-dsl";
import { CHANNELS, type Channel, type Conversation, type History, type Media } from "./api.js";
import { IPC, type ConsoleAttachment, type ConsoleDashboard, type ConsoleRequest, type Manage, type ManagementStatus } from "./service.js";
import { consoleLayout, platforms } from "./console-layout.js";
import qr from "./vendor/qrcode.js";

type View = "overview" | "conversations" | Channel;
const tabs: readonly View[] = ["overview", "conversations", ...CHANNELS];
const label = (view: View) => view === "overview" ? "总览" : view === "conversations" ? "会话" : platforms[view].short;
const mediaLabel = { image: "图片", audio: "语音", file: "文件", video: "视频" };
const call = <T>(request: ConsoleRequest) => ToolPkg.ipc.call<ConsoleRequest, T>(IPC, request, { targetRuntime: "main" });
function filePayload(channel: Channel, file: ComposePickedFile): Media {
  const mime = file.mimeType ?? "";
  const audio = channel === "qq" || channel === "feishu" && /\.(opus|ogg)$/i.test(file.name ?? "");
  const video = channel !== "feishu" || /\.mp4$/i.test(file.name ?? "");
  const kind = mime.startsWith("image/") ? "image" : mime.startsWith("audio/") && audio ? "audio" : mime.startsWith("video/") && video ? "video" : "file";
  return { kind, source: file.path, fileName: file.name };
}

export default function consoleScreen(ctx: ComposeDslContext): ComposeNode {
  const ui = ctx.UI;
  const [data, setData] = ctx.useState<ConsoleDashboard | null>("dashboard", null);
  const [view, setView] = ctx.useState<View>("view", "overview");
  const [width, setWidth] = ctx.useState("width", 0);
  const [changingAccount, setChangingAccount] = ctx.useState("changingAccount", false);
  const [confirmLogout, setConfirmLogout] = ctx.useState("confirmLogout", false);
  const [login, setLogin] = ctx.useState<NonNullable<ManagementStatus["login"]> | null>("login", null);
  const [appId, setAppId] = ctx.useState("appId", "");
  const [secret, setSecret] = ctx.useState("secret", "");
  const [code, setCode] = ctx.useState("code", "");
  const [query, setQuery] = ctx.useState("query", "");
  const [filter, setFilter] = ctx.useState<Channel | null>("filter", null);
  const [selected, select] = ctx.useState<Conversation | null>("selected", null);
  const [history, setHistory] = ctx.useState<History | null>("history", null);
  const [reply, setReply] = ctx.useState<string | null>("reply", null);
  const [draft, setDraft] = ctx.useState("draft", "");
  const [media, setMedia] = ctx.useState<Media | null>("media", null);
  const [notice, setNotice] = ctx.useState("notice", "");
  const [error, setError] = ctx.useState("error", "");
  const [busy, setBusy] = ctx.useState("busy", false);
  const working = ctx.useRef("working", false);

  async function refresh(): Promise<void> { setData(await call<ConsoleDashboard>({ action: "dashboard" })); }
  async function run(work: () => Promise<void>): Promise<void> {
    if (working.current) return;
    working.current = true; setBusy(true); setNotice(""); setError("");
    try { await work(); }
    catch (error) { setError(error instanceof Error ? error.message : String(error)); }
    finally { working.current = false; setBusy(false); }
  }
  async function load(conversation: Conversation, before?: number): Promise<void> {
    const next = await call<History>({ action: "history", channel: conversation.channel, conversationKey: conversation.key, limit: 20, before });
    setHistory(before === undefined ? next : { ...next, messages: [...next.messages, ...(history?.messages ?? [])].slice(0, 100) });
  }
  async function manage(command: Manage): Promise<void> {
    const status = await call<ManagementStatus>(command);
    if (status.login) setLogin(status.login);
    if (command.command === "connect") { setSecret(""); setAppId(""); setChangingAccount(false); }
    if (command.command === "logout") setConfirmLogout(false);
    if (command.command === "login_verify") setCode("");
    await refresh();
    if (command.command === "test") setNotice("连接校验通过");
  }
  async function show(next: View): Promise<void> {
    if (working.current || next === view) return;
    setView(next); setNotice(""); setError("");
    setAppId(""); setSecret(""); setCode(""); setLogin(null); setChangingAccount(false); setConfirmLogout(false);
    if (next === "wechat" && data?.connectors.some(connector => connector.channel === next && connector.available)) {
      await run(async () => setLogin((await call<ManagementStatus>({ action: "manage", channel: next, command: "login_status" })).login ?? null));
    }
  }
  ctx.useEffect("refresh", () => {
    let active = true;
    let timer: number | null = null;
    const tick = async () => {
      try {
        if (!working.current) {
          if (view === "wechat" && login && ["wait", "scaned", "scaned_but_redirect", "need_verifycode"].includes(login.state)) {
            const result = await call<ManagementStatus>({ action: "manage", channel: "wechat", command: "login_status" });
            if (active && !working.current && result.login) setLogin(result.login);
          }
          const next = await call<ConsoleDashboard>({ action: "dashboard" });
          if (active && !working.current) setData(next);
        }
      } catch (error) { if (active && !working.current) setError(error instanceof Error ? error.message : String(error)); }
      finally { if (active) timer = setTimeout(() => void tick(), view === "wechat" && login?.qrContent ? 1800 : 6000); }
    };
    timer = setTimeout(() => void tick(), 1500);
    return () => { active = false; if (timer !== null) clearTimeout(timer); };
  }, [view, login?.state, login?.qrContent]);

  const { colors, shape, logo, text, muted: sub, icon, row, flow, stack: column, badge, tile, status: stateBadge, button, card, empty } = consoleLayout(ctx, busy);
  const field = (key: string, label: string, value: string, onValueChange: (val: string) => void, options: { isPassword?: boolean; singleLine?: boolean } = {}) =>
    ui.OutlinedTextField({ key, label, value, onValueChange, fillMaxWidth: true, enabled: !busy, singleLine: true, ...options });
  const heading = (value: string) => text(value, "titleMedium");
  const formatTime = (ms: number) => !Number.isFinite(ms) || ms <= 0 ? "" : new Date(ms).toLocaleString();
  const qrSvg = ctx.useMemo("qr", () => {
    if (!login?.qrContent) return "";
    try { const matrix = qr(0, "M"); matrix.addData(login.qrContent, "Byte"); matrix.make(); return matrix.createSvgTag({ cellSize: 4, margin: 16, scalable: true }); }
    catch { return ""; }
  }, [login?.qrContent]);

  const nodes: ComposeNode[] = [];
  const closeConversation = () => { select(null); setHistory(null); setReply(null); setDraft(""); setMedia(null); setNotice(""); setError(""); };
  const conversations = data?.conversations ?? [];
  const connectorFor = (channel: Channel) => data?.connectors.find(connector => connector.channel === channel);
  const openConversation = (item: Conversation) => run(async () => {
    await load(item); select(item); setReply(null); setDraft(""); setMedia(null); setView("conversations");
  });
  const openList = (channel: Channel | null) => { closeConversation(); setQuery(""); setFilter(channel); return show("conversations"); };
  const sessionRow = (item: Conversation) => ui.Surface({ key: `open-${item.key}`, fillMaxWidth: true, shape,
    containerColor: colors.surfaceContainerLow, ...(busy ? {} : { onClick: () => openConversation(item) }) },
  ui.Column({ padding: { horizontal: 12, vertical: 10 }, fillMaxWidth: true }, row(`session-${item.key}`, [tile(item.channel, 32),
    ui.Column({ weight: 1, spacing: 4 }, [text(item.title, "titleSmall", { maxLines: 1, overflow: "ellipsis" }),
      sub([platforms[item.channel].short, item.scene === "group" ? "群聊" : "私聊", formatTime(item.lastMessageAt)].filter(Boolean).join(" · "))]),
    icon("ChevronRight", 18, colors.onSurfaceVariant)])));

  if (view === "conversations" && selected) {
    const target = reply ?? history?.latestReplyKey ?? [...(history?.messages ?? [])].reverse().find(item => item.replyKey)?.replyKey;
    const canSend = !!target || selected.channel === "feishu";
    nodes.push(row("conversation-title", [ui.IconButton({ key: "back", icon: "ArrowBack", tooltip: "返回会话列表", enabled: !busy, onClick: closeConversation }),
      ui.Column({ weight: 1, spacing: 3 }, [text(selected.title, "titleLarge", { maxLines: 2, overflow: "ellipsis" }), sub(platforms[selected.channel].name)])]));
    if (history?.truncated && history.messages.length < 100 && history.messages[0]) nodes.push(flow("older-actions", [button("older", "加载更早消息", () => run(() => load(selected, history.messages[0].receivedAt)), "text", true, "History")]));
    if (!history?.messages.length) nodes.push(empty("empty-history", "ChatBubbleOutline", "暂无近期消息"));
    for (const message of history?.messages ?? []) {
      const outgoing = message.direction === "outgoing";
      const sender = message.authorName || (outgoing ? "机器人" : "收到的消息");
      const delivery = !outgoing ? null : message.delivery === "rejected" ? badge("平台拒绝", colors.onErrorContainer, colors.errorContainer) :
        message.delivery === "unknown" ? badge("发送结果未知") : badge("平台已接受");
      const attachments = (message.media ?? []).map((value, index) => {
        const label = value.name || mediaLabel[value.kind];
        if (message.direction === "outgoing" || !value.downloadable) return sub(label);
        return flow(`attachment-${message.messageId}-${index}`, [sub(label), button(`save-${message.messageId}-${index}`, `保存${mediaLabel[value.kind]}`, () => run(async () => {
          const { fileName, mimeType, contentBase64 } = await call<ConsoleAttachment>({ action: "attachment", channel: selected.channel,
            conversationKey: selected.key, messageId: message.messageId, index });
          const suggestedName = fileName && fileName !== "." && fileName !== ".." ? fileName : "附件";
          const extension = /\.([a-z0-9]{1,12})$/i.exec(suggestedName)?.[1];
          const result = await ctx.saveFile({suggestedName, mimeType, contentBase64, extensions: extension ? [extension] : []});
          if (!result.cancelled) setNotice("附件已保存");
        }), "text", true, "Download")]);
      });
      const quoted = !!reply && reply === message.replyKey;
      nodes.push(ui.Box({ key: `message-${message.messageId}`, fillMaxWidth: true, contentAlignment: outgoing ? "end" : "start" },
        ui.Column({ modifier: ctx.Modifier.widthIn({ maxWidth: 680 }), padding: 12, spacing: 6, backgroundShape: shape,
          background: outgoing ? colors.secondaryContainer : colors.surfaceContainerLow }, [
        flow(`metadata-${message.messageId}`, [text(sender, "labelLarge", { maxLines: 1, overflow: "ellipsis" }), sub(formatTime(message.receivedAt)), ...(delivery ? [delivery] : [])]),
        ...(message.text ? [text(message.text)] : []), ...attachments,
        ...(message.replyKey ? [button(`reply-${message.messageId}`, quoted ? "取消引用" : "引用回复", () => setReply(quoted ? null : message.replyKey!), quoted ? "tonal" : "text", true, "Reply")] : []),
        ...(message.delivery === "rejected" && message.text && !message.media?.length && !message.text.startsWith("[") && !draft && !media ?
          [button(`draft-${message.messageId}`, "恢复为草稿", () => setDraft(message.text))] : [])])));
    }
    if (canSend) {
      const quoted = reply ? history?.messages.find(message => message.replyKey === reply) : undefined;
      const composer: ComposeNode[] = [];
      if (reply) composer.push(flow("quote", [icon("Reply", 16), text(quoted?.text || "已选择引用消息", "bodySmall", { maxLines: 1, overflow: "ellipsis" }), button("clear-quote", "取消引用", () => setReply(null))]));
      composer.push(field("draft", target ? "回复内容" : "消息内容", draft, setDraft, { singleLine: false }));
      if (media) composer.push(sub(`${mediaLabel[media.kind]}：${media.fileName ?? "已选择"}`));
      if (media?.kind === "video" && selected.channel === "feishu") composer.push(button("cover", media.cover ? "更换封面" : "添加封面（可选）", () => run(async () => {
        const picked = await ctx.openFilePicker({ picker: "image" }); if (picked.files[0]) setMedia({ ...media, cover: picked.files[0].path });
      })));
      composer.push(flow("send-actions", [button("attachment", media ? "移除附件" : "添加附件", () => run(async () => {
        if (media) { setMedia(null); return; }
        const picked = await ctx.openFilePicker({ picker: "document" });
        if (picked.files[0]) setMedia(filePayload(selected.channel, picked.files[0]));
      }), "text", true, media ? "Close" : "AttachFile"), button("send", media ? "发送附件" : "发送", () => run(async () => {
        const fields = media ? { media } : { text: draft.trim() };
        try {
          await call(target ? { action: "console_reply", channel: selected.channel, conversationKey: selected.key, key: target, ...fields }
            : { action: "send", channel: selected.channel, conversationKey: selected.key, ...fields });
        } catch (error) {
          try { await load(selected); } catch { /* Preserve the original send error. */ }
          throw error;
        }
        setReply(null); if (media) setMedia(null); else setDraft("");
        setNotice("平台已接受");
        try { await load(selected); } catch { setNotice("已发送，消息记录刷新失败"); }
      }), "primary", !!(media || draft.trim()), "Send")]));
      nodes.push(card("composer", composer));
    }
  } else if (view === "overview") {
    const wide = width >= 720;
    const cards = CHANNELS.map(channel => {
      const connector = connectorFor(channel);
      const count = conversations.filter(item => item.channel === channel).length;
      const action = button(`settings-${channel}`, !connector?.available ? "查看" : connector.status?.configured ? "管理" : "连接",
        () => show(channel), connector?.available && !connector.status?.configured ? "primary" : "tonal");
      return card(`channel-${channel}`, [row(`connector-${channel}`, [tile(channel),
        ui.Column({ weight: 1, spacing: 4 }, [text(platforms[channel].name, "titleSmall", { fontWeight: "bold" }), sub(`${count} 个会话`)]),
        ...(!wide ? [action] : [])]),
        flow(`status-${channel}`, [stateBadge(connector, !!data), ...(wide ? [action] : [])])]);
    });
    nodes.push(text("机器人", "titleMedium"));
    if (wide) nodes.push(row("channel-grid", cards.map((item, index) => ui.Box({ key: CHANNELS[index], weight: 1 }, item))));
    else nodes.push(...cards);
    nodes.push(row("recent-heading", [text("最近会话", "titleMedium", { weight: 1 }),
      button("all-conversations", "全部会话", () => openList(null), "text", true, "ChevronRight")]));
    if (conversations.length) nodes.push(...conversations.slice(0, 5).map(sessionRow));
    else nodes.push(empty("no-recent-conversations", "Inbox", data ? "还没有收到消息" : error ? "会话读取失败" : "正在读取会话"));
  } else if (view === "conversations") {
    const search = query.trim().toLocaleLowerCase();
    const visible = conversations.filter(item => (!filter || item.channel === filter) && item.title.toLocaleLowerCase().includes(search)).slice(0, 100);
    nodes.push(text("会话", "titleLarge"), field("search", "搜索会话", query, setQuery),
      ui.SegmentedControl({ key: "filters", value: filter ?? "all", enabled: !busy,
        options: [{ value: "all", label: "全部" }, ...CHANNELS.map(value => ({ value, label: platforms[value].short }))],
        onValueChange: (value: string) => setFilter(CHANNELS.find(channel => channel === value) ?? null) }));
    if (!visible.length) nodes.push(empty("no-conversations", search ? "Search" : "Inbox", !data ? error ? "会话读取失败" : "正在读取会话" : search ? "没有匹配的会话" : "还没有收到消息"));
    else nodes.push(...visible.map(sessionRow));
  } else {
    const channel = view;
    const { settings: nativeSettings, name } = platforms[channel];
    const connector = connectorFor(channel);
    const status = connector?.status;
    const panels: ComposeNode[] = [];
    nodes.push(row("bot-title", [tile(channel), ui.Column({ weight: 1, spacing: 4 }, [text(name, "titleLarge"), stateBadge(connector, !!data)])]));
    const shortcuts: ComposeNode[] = [button(`conversations-${channel}`, "查看会话", () => openList(channel), "tonal", true, "Forum")];
    if (nativeSettings && connector?.available) shortcuts.push(button(`native-${channel}`, "高级设置", () => run(async () => { await ctx.navigate(nativeSettings); }), "text", true, "Settings"));
    if (data && !connector?.available) shortcuts.push(button("extensions", "打开包管理", () => run(async () => { await ctx.navigate("native.package_manager"); }), "text", true, "Extension"));
    nodes.push(flow("platform-shortcuts", shortcuts));
    if (connector?.issue || status?.error) nodes.push(sub(connector?.issue || status?.error || ""));
    if (status && connector?.available) {
      const controls: ComposeNode[] = [heading("连接")];
      if (channel === "wechat") {
        if (!status.configured || (login && !["idle", "confirmed"].includes(login.state))) {
          controls.push(button("qr-start", qrSvg ? "重新获取二维码" : "获取登录二维码", () => run(() => manage({ action: "manage", channel, command: "login_start" })), "primary", true, "QrCode"));
          if (qrSvg) controls.push(ui.Svg({ key: "qr", source: qrSvg, width: 204, height: 204 }),
            sub(login?.state === "need_verifycode" ? "需要验证码" : login?.state === "scaned" ? "已扫码，等待微信确认" : "等待扫码"),
            button("qr-cancel", "取消扫码", () => run(() => manage({ action: "manage", channel, command: "login_cancel" }))));
          if (login?.state === "need_verifycode") controls.push(field("verify", "微信验证码", code, setCode, { isPassword: true }),
            button("submit-code", "提交验证码", () => run(() => manage({ action: "manage", channel, command: "login_verify", code })), "primary", !!code.trim()));
          if (login?.error) controls.push(sub(login.error));
        } else {
          controls.push(flow("wechat-account", [button("qr-renew", "重新扫码", () => run(() => manage({ action: "manage", channel, command: "login_start" }))),
            button("logout", confirmLogout ? "确认退出登录" : "退出登录", () => confirmLogout ? run(() => manage({ action: "manage", channel, command: "logout" })) : setConfirmLogout(true))]));
        }
      } else if (!status.configured || changingAccount) {
        controls.push(field("app-id", "App ID", appId, setAppId), field("secret", "App Secret", secret, setSecret, { isPassword: true }),
          button("connect", "校验并连接", () => run(() => manage({ action: "manage", channel, command: "connect", appId, appSecret: secret })), "primary", !!(appId.trim() && secret.trim()), "Link"));
        if (changingAccount) controls.push(button("cancel-change", "取消", () => { setChangingAccount(false); setAppId(""); setSecret(""); }));
      } else controls.push(button("change-account", "更换应用凭据", () => setChangingAccount(true)));
      if (status.configured) controls.push(flow("controls", [
        button("toggle", status.listenerEnabled ? "停止监听" : "启动监听", () => run(() => manage({ action: "manage", channel, command: status.listenerEnabled ? "stop" : "start" })), "tonal"),
        button("test", "测试连接", () => run(() => manage({ action: "manage", channel, command: "test" })))]));
      panels.push(card("connection-controls", [column(`settings-${channel}-body`, controls, 12)]));
      if (status.configured) {
        const replies: ComposeNode[] = [heading("自动回复"),
          row("auto", [text(channel === "feishu" ? "AI 自动回复（私聊）" : "AI 自动回复", "bodyMedium", { weight: 1 }), ui.Switch({ checked: status.autoReplyEnabled === true,
            enabled: !busy && status.listenerEnabled === true, onCheckedChange: enabled => { void run(() => manage({ action: "manage", channel, command: "start", autoReply: enabled })); } })])];
        if (channel === "qq" && status.replyActivity) {
          const activity = status.replyActivity;
          const group = activity.groupMode === "mentioned" ? "群聊仅被 @ 时回复" : activity.groupMode === "all" ? "群聊回复所有消息" : "群聊回复关闭";
          replies.push(sub(`${activity.directEnabled ? "私聊回复开启" : "私聊回复关闭"} · ${group}`),
            sub(`本次运行收到 ${activity.received} 条 · 已回复 ${activity.replied} 条`));
          if (status.autoReplyEnabled && !activity.running) replies.push(sub("自动回复等待监听恢复"));
          if (activity.failed) replies.push(sub(`${activity.failed} 条回复未完成，可在高级设置的诊断信息中查看。`));
        }
        if (channel === "feishu" && typeof status.groupAutoReplyEnabled === "boolean") {
          replies.push(row("group-auto", [text("AI 自动回复（群内 @ Bot）", "bodyMedium", { weight: 1 }), ui.Switch({ checked: status.groupAutoReplyEnabled,
            enabled: !busy, onCheckedChange: enabled => { void run(() => manage({ action: "manage", channel, command: "group_auto_reply", enabled })); } })]));
          if (status.groupAutoReplyIssue) replies.push(sub(status.groupAutoReplyIssue));
        }
        panels.push(card("reply-controls", [column("reply-settings", replies)]));
      }
    }
    if (width >= 760 && panels.length > 1 && !(channel === "wechat" && login?.qrContent)) {
      nodes.push(ui.Row({ key: "bot-panels", fillMaxWidth: true, verticalAlignment: "start", spacing: 12 },
        panels.map((panel, index) => ui.Box({ key: index === 0 ? "connection" : "reply", weight: 1 }, panel))));
    } else {
      nodes.push(...panels);
    }
  }
  const refreshControl = ui.IconButton({ key: "refresh", content: icon("Refresh", 20, "#DCE7FF"), tooltip: "刷新", enabled: !busy,
    onClick: () => run(async () => { await refresh(); if (view === "conversations" && selected) await load(selected); }) });
  const feedback = error || notice;
  const page = view === "conversations" && selected ? `thread-${selected.key}` : view;
  const inset = { horizontal: 18, vertical: 14 };
  const online = data?.connectors.filter(connector => connector.available && connector.status?.service === "ready").length ?? 0;
  const overviewStatus = data ? `${online} 个在线 · ${conversations.length} 个会话` : "正在读取连接状态";
  return ui.Column({ fillMaxSize: true, horizontalAlignment: "center", onLoad: () => run(refresh) }, [
    ui.Box({ fillMaxWidth: true, modifier: ctx.Modifier.widthIn({ maxWidth: 1040 }), padding: { horizontal: 16, vertical: 10 } },
      ui.Column({ fillMaxWidth: true, padding: inset, background: "#17264D", backgroundShape: { cornerRadius: 18 } }, [
        row("title", [logo(44), ui.Column({ weight: 1, spacing: 4 }, [
          text("外部连接器", "titleLarge", { color: "#F4F7FF", fontWeight: "bold" }),
          text(overviewStatus, "bodySmall", { color: "#C0D0F5" }),
        ]), ...(busy ? [ui.CircularProgressIndicator({ width: 16, height: 16, strokeWidth: 2, color: "#77E7C3" })] : []), refreshControl]),
      ])),
    ui[width > 0 && width < 320 ? "SecondaryScrollableTabRow" : "SecondaryTabRow"]({ key: "navigation",
      fillMaxWidth: true, modifier: ctx.Modifier.widthIn({ maxWidth: 1024 }), selectedTabIndex: tabs.indexOf(view),
      tabs: tabs.map(tab => ui.Tab({ key: `tab-${tab}`, selected: view === tab, enabled: !busy, onClick: () => show(tab) },
        ui.Text({ text: label(tab), inheritStyle: true, maxLines: 1, overflow: "ellipsis", letterSpacing: 0 }))),
    }),
    ...(feedback ? [ui.Column({ fillMaxWidth: true, padding: inset, background: error ? colors.errorContainer : colors.secondaryContainer,
      modifier: ctx.Modifier.widthIn({ maxWidth: 1024 }) }, [text(feedback, "bodySmall", { color: error ? colors.onErrorContainer : colors.onSecondaryContainer })])] : []),
    ui.LazyColumn({ key: page, fillMaxWidth: true, weight: 1, padding: { horizontal: 16 }, contentPadding: { vertical: 12 }, spacing: 10,
      modifier: ctx.Modifier.widthIn({ maxWidth: 1024 }).onSizeChanged(size => { if (size.width !== width) setWidth(size.width); }) }, nodes),
  ]);
}
