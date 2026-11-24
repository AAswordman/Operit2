import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Chat } from "../../../../types/chat";
import { characterCards, type ChatDisplayIdentity } from "./dependencies/character_cards/api.js";

const PACKAGE = "com.operit.voice_call";
const CONTROL_ROUTE = `toolpkg:${PACKAGE}:ui:control`;
const WINDOW = "voice-call";
const title = { zh: "语音通话", en: "Voice Call" };
const RECORDING_OPTIONS = { maxDurationMs: 20000, silenceMs: 900 };

const labels = {
  idle: ["准备通话", "Ready to call"],
  connecting: ["正在建立会话…", "Opening conversation…"],
  listening: ["正在听…", "Listening…"],
  recognizing: ["正在识别…", "Transcribing…"],
  waiting: ["正在等待回复…", "Waiting for a reply…"],
  speaking: ["正在播报…", "Speaking…"],
  closing: ["正在挂断…", "Ending call…"],
  error: ["通话已暂停", "Call paused"],
  start: ["开始通话", "Start call"],
  retry: ["继续通话", "Resume call"],
  submit: ["说完了", "Done speaking"],
  interrupt: ["打断并说话", "Interrupt and speak"],
  hangup: ["挂断", "Hang up"],
  open: ["打开通话窗", "Open call window"],
  voiceOverlay: ["打开系统语音悬浮窗", "Open system voice overlay"],
  grant: ["授予悬浮窗权限", "Grant overlay permission"],
  windowNotReady: ["通话窗口尚未就绪，请重试。", "Call window is not ready. Please try again."],
  unsupported: ["当前宿主不支持悬浮窗。", "Floating windows are unavailable on this host."],
  permission: ["在系统设置授权后，再打开通话窗。", "Grant permission in system settings, then open the call window."],
  setup: ["使用应用当前的语音识别和朗读配置。每个通话窗使用一个独立会话。", "Uses the selected speech recognition and voice settings. Each call window gets its own conversation."],
  system: ["已打开系统悬浮窗。", "Opened the system overlay."],
  application: ["已打开 Operit 应用内悬浮窗。", "Opened a floating window inside Operit."],
  noReply: ["本轮没有可播报的 AI 回复。", "No assistant reply was available to speak."],
  cameraFrameFailed: ["本轮未能发送摄像头画面。", "Couldn't send the camera frame for this turn."],
  you: ["你", "You"],
} as const;

type Label = keyof typeof labels;
type Phase = "idle" | "connecting" | "listening" | "recognizing" | "waiting" | "speaking" | "closing" | "error";
type Playback = { id: string; done: boolean; failed: boolean; cancellation?: Promise<boolean> };

/** Selects window and control text using the plugin locale. */
function text(label: Label): string {
  return labels[label][getLang().startsWith("zh") ? 0 : 1];
}

/** Formats and logs a host error without hiding its reason from the user. */
function report(error: unknown): string {
  console.error("[voice_call]", error);
  return error instanceof Error ? error.message : String(error);
}

/** Uses one public route for the launcher and its plugin-owned floating content. */
export function screen(ctx: ComposeDslContext): ComposeNode {
  const [callSurface] = ctx.useState("callSurface", false);
  return callSurface ? call(ctx) : control(ctx);
}

/** Chooses camera-compatible presentation using public host capabilities. */
async function cameraWindowCapabilities(): Promise<ToolPkg.FloatingWindowCapabilities> {
  const api = ToolPkg.ui.floatingWindows;
  const native = await api.capabilities();
  return native.cameraRequiresApplicationWindow
    ? api.capabilities({ applicationOnly: true }) : native;
}

/** Opens or reveals the single call window and handles overlay authorization. */
export function control(ctx: ComposeDslContext): ComposeNode {
  const windowApi = ToolPkg.ui.floatingWindows;
  const [chatId] = ctx.useState<string | null>("chatId", null);
  const [status, setStatus] = ctx.useState("status", "");
  const [needsPermission, setNeedsPermission] = ctx.useState("needsPermission", false);
  const [busy, setBusy] = ctx.useState("busy", false);
  const [offersVoiceOverlay, setOffersVoiceOverlay] = ctx.useState("offersVoiceOverlay", false);
  const inFlight = ctx.useRef("inFlight", false);

  ctx.useEffect("voice-overlay-capability", () => {
    let disposed = false;
    void cameraWindowCapabilities().then(caps => {
      if (!disposed) setOffersVoiceOverlay(caps.scope === "application" && caps.systemOverlay === true);
    }).catch(report);
    return () => { disposed = true; };
  }, []);

  /** Runs an explicit window operation and reflects its outcome. */
  async function launch(voiceOverlay: boolean): Promise<void> {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    try {
      const caps = await cameraWindowCapabilities();
      if (!caps.supported) throw new Error(text("unsupported"));
      setNeedsPermission(!caps.permissionGranted);
      if (!caps.permissionGranted) {
        setStatus(text("permission"));
        return;
      }
      const windows = await windowApi.list();
      const existing = windows.find(window => window.id === WINDOW);
      if (existing) {
        await windowApi.show(WINDOW);
      } else {
        if (voiceOverlay && !await windowApi.requestPermission()) {
          setStatus(text("permission"));
          return;
        }
        await windowApi.open({
          id: WINDOW,
          applicationOnly: !voiceOverlay && caps.cameraRequiresApplicationWindow === true,
          state: { callSurface: true, cameraControls: !voiceOverlay, sourceChatId: chatId },
          route: CONTROL_ROUTE,
          options: { width: 340, height: voiceOverlay ? 404 : 460, draggable: true },
        });
      }
      const system = caps.scope === "system" || voiceOverlay;
      setStatus(text(system ? "system" : "application"));
    } catch (error) {
      setStatus(report(error));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  }

  /** Opens the platform authorization UI from a user gesture. */
  async function requestPermission(): Promise<void> {
    try {
      await windowApi.requestPermission();
    } catch (error) {
      setStatus(report(error));
    }
  }

  return ctx.UI.Column({ padding: 16, spacing: 10 }, [
    ctx.UI.Text({ text: text("setup"), style: "bodyMedium" }),
    ctx.UI.Button({ text: text("open"), enabled: !busy, onClick: () => launch(false) }),
    ...(offersVoiceOverlay ? [ctx.UI.OutlinedButton({ enabled: !busy, onClick: () => launch(true) }, [
      ctx.UI.Text({ text: text("voiceOverlay") }),
    ])] : []),
    ...(needsPermission ? [ctx.UI.OutlinedButton({ onClick: requestPermission }, [ctx.UI.Text({ text: text("grant") })])] : []),
    ...(status ? [ctx.UI.Text({ text: status, style: "bodySmall" })] : []),
  ]);
}

/** Runs the plugin-owned speech and model conversation. */
export function call(ctx: ComposeDslContext): ComposeNode {
  const windowApi = ToolPkg.ui.floatingWindows;
  const [autoStart] = ctx.useState("autoStart", false);
  const [sourceChatId] = ctx.useState<string | null>("sourceChatId", null);
  const [cameraControls] = ctx.useState("cameraControls", true);
  const [phase, setPhase] = ctx.useState<Phase>("phase", "idle");
  const [heard, setHeard] = ctx.useState("heard", "");
  const [reply, setReply] = ctx.useState("reply", "");
  const [failure, setFailure] = ctx.useState("failure", "");
  const [cameraNotice, setCameraNotice] = ctx.useState("cameraNotice", "");
  const [identity, setIdentity] = ctx.useState<ChatDisplayIdentity | null>("identity", null);
  const identityRequest = ctx.useRef("identityRequest", 0);
  const session = ctx.useRef("session", {
    chatId: "",
    active: false,
    closing: false,
    sending: false,
    playback: null as Playback | null,
    recording: false,
    groupRequest: "",
    skipReplies: false,
  }).current;

  async function interruptSpeech(): Promise<void> {
    session.skipReplies = true;
    const playback = session.playback;
    if (playback && !playback.done) {
      const cancellation = playback.cancellation ??= Tools.Tts.cancel(playback.id).catch(error => {
        if (session.active) setFailure(report(error));
        else report(error);
        return false;
      });
      if (!await cancellation && playback.cancellation === cancellation) playback.cancellation = undefined;
    }
  }

  /** Prevents another turn and releases resources already in use. */
  async function stopCall(): Promise<void> {
    session.active = false;
    await Promise.all<unknown>([
      ...(session.groupRequest ? [characterCards.chat.groupTurn({ action: "cancel", chatId: session.chatId, requestId: session.groupRequest }).catch(report)]
        : session.sending ? [Tools.Chat.cancel(session.chatId).catch(report)] : []),
      interruptSpeech(),
      ...(session.recording ? [windowApi.cancelRecording(WINDOW).catch(report)] : []),
    ]);
  }

  ctx.useEffect("call-lifetime", () => () => { void stopCall(); }, []);

  /** Uses the same default artwork as the required character card plugin. */
  async function readIdentity(chatId: string | null): Promise<ChatDisplayIdentity> {
    const value = await characterCards.chat.identity({ chatId });
    return {
      title: value?.title ?? "Operit",
      avatarUri: value?.avatarUri || await characterCards.defaultAvatar(),
    };
  }

  ctx.useEffect("call-avatar", () => {
    let disposed = false;
    const request = ++identityRequest.current;
    void readIdentity(sourceChatId).then(value => {
      if (!disposed && request === identityRequest.current) setIdentity(value);
    }).catch(error => {
      if (!disposed && request === identityRequest.current) setFailure(report(error));
    });
    return () => { disposed = true; };
  }, []);

  /** Retains the temporary camera selection before placing it in chat history. */
  async function cameraAttachment(): Promise<Chat.SendAttachment | null> {
    if (typeof windowApi.captureCameraFrame !== "function") return null;
    const frame = await windowApi.captureCameraFrame(WINDOW);
    if (!frame) return null;
    let path = "";
    let retained = false;
    try {
      if (!session.active) return null;
      const directory = `${ToolPkg.getSpaceDataDir()}/camera_frames`;
      const created = await Tools.Files.mkdir(directory, true);
      if (!created.successful) throw new Error(created.details);
      path = `${directory}/frame-${Date.now()}-${Math.random().toString(36).slice(2)}.jpg`;
      const copied = await Tools.Files.copy(frame.path, path);
      if (!copied.successful) throw new Error(copied.details);
      if (!session.active) return null;
      retained = true;
      return {
        vfsPath: path, filePath: path, fileName: frame.name,
        mimeType: frame.mimeType, fileSize: frame.size, content: "",
      };
    } finally {
      await Tools.Files.deleteFile(frame.path).catch(report);
      if (path && !retained) await Tools.Files.deleteFile(path).catch(report);
    }
  }

  async function record(): Promise<ToolPkg.FloatingWindowRecording | null> {
    session.recording = true;
    try { return await windowApi.record(WINDOW, RECORDING_OPTIONS); }
    finally { session.recording = false; }
  }

  /** Leaves the microphone idle until playback finishes or the user presses interrupt. */
  async function speak(answer: string, id: string, configId?: string): Promise<void> {
    setPhase("speaking");
    const playback: Playback = { id, done: false, failed: false };
    session.playback = playback;
    try {
      await Tools.Tts.speak({ text: answer, interrupt: false, waitForCompletion: true,
        playbackId: playback.id, ...(configId ? { configId } : {}),
      }).catch(async error => {
        if (await playback.cancellation) return;
        playback.failed = true;
        throw error;
      });
    } finally {
      playback.done = true;
      session.playback = null;
    }
  }

  /** Reuses the character package's group planner and exact committed replies. */
  async function groupTurn(transcript: string, attachments: Chat.SendAttachment[]): Promise<boolean> {
    const requestId = `call-${session.chatId}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    session.groupRequest = requestId;
    session.skipReplies = false;
    let spoken = 0;
    try {
      let progress = await characterCards.chat.groupTurn({ action: "start", chatId: session.chatId,
        requestId, text: transcript, attachments });
      while (session.active) {
        if (progress.status === "not_group") return false;
        for (; spoken < progress.replies.length && session.active && !session.skipReplies; spoken++) {
          const reply = progress.replies[spoken];
          setIdentity({ title: reply.name, avatarUri: reply.avatarUri || await characterCards.defaultAvatar() });
          setReply(reply.text);
          await speak(reply.text, `${requestId}-${spoken}`, reply.ttsConfigId ?? undefined);
        }
        if (!session.active || session.skipReplies) {
          await characterCards.chat.groupTurn({ action: "cancel", chatId: session.chatId, requestId });
          return true;
        }
        if (progress.status === "failed") throw new Error(progress.error || text("noReply"));
        if (progress.status === "completed") {
          if (spoken === 0) throw new Error(text("noReply"));
          return true;
        }
        if (progress.status === "cancelled") return true;
        setPhase("waiting");
        await new Promise<void>(resolve => setTimeout(resolve, 200));
        if (!session.active) break;
        progress = await characterCards.chat.groupTurn({ action: "status", chatId: session.chatId, requestId });
      }
      await characterCards.chat.groupTurn({ action: "cancel", chatId: session.chatId, requestId });
      return true;
    } catch (failure) {
      await characterCards.chat.groupTurn({ action: "cancel", chatId: session.chatId, requestId }).catch(report);
      throw failure;
    } finally { session.groupRequest = ""; }
  }

  /** Records each turn, persists it in chat, and plays the reply. */
  async function run(): Promise<void> {
    if (session.active || session.closing) return;
    session.active = true;
    ++identityRequest.current;
    setFailure("");
    setCameraNotice("");
    try {
      setPhase("connecting");
      if (!session.chatId) {
        const chat = await Tools.Chat.createNew({ setAsCurrentChat: false, ...(sourceChatId ? { sourceChatId } : {}) });
        session.chatId = chat.chatId;
      }
      const currentIdentity = await readIdentity(session.chatId);
      if (!session.active) return;
      setIdentity(currentIdentity);
      while (session.active) {
        setIdentity(currentIdentity);
        setPhase("listening");
        const recording = await record();
        if (!session.active) return;
        if (!recording) {
          setPhase("idle");
          return;
        }
        setPhase("recognizing");
        const recognized = await Tools.Stt.transcribe(recording);
        if (!session.active) return;
        const transcript = recognized.text.trim();
        if (!transcript) continue;

        setHeard(transcript);
        setReply("");
        setPhase("waiting");
        setCameraNotice("");
        const attachment = await cameraAttachment().catch(error => {
          if (session.active) setCameraNotice(text("cameraFrameFailed"));
          report(error);
          return null;
        });
        if (!session.active) return;
        if (await groupTurn(transcript, attachment ? [attachment] : [])) continue;
        if (!session.active) return;
        session.sending = true;
        const turn = await Tools.Chat.sendMessage({
          kind: "submit",
          chatId: session.chatId,
          runtime: "floating",
          input: { text: transcript, attachments: attachment ? [attachment] : [], replyToMessageTimestamp: null },
          turn: { kind: "execute" },
          notifyReply: false,
        }).finally(() => { session.sending = false; });
        if (!session.active) return;
        if (turn.outcome.type === "blocked") {
          throw new Error(turn.outcome.message ?? text("noReply"));
        }
        const answer = turn.aiResponse?.trim();
        if (!answer) throw new Error(text("noReply"));

        setReply(answer);
        session.skipReplies = false;
        await speak(answer, `call-${session.chatId}-${turn.sentAt}`);
      }
    } catch (error) {
      if (session.active) {
        setFailure(report(error));
        setPhase("error");
      }
    } finally {
      session.active = false;
    }
  }

  /** Ends the active recording and lets the current turn continue. */
  async function finishRecording(): Promise<void> {
    try {
      await windowApi.finishRecording(WINDOW);
    } catch (error) {
      setFailure(report(error));
    }
  }

  /** Prevents later phases from starting and closes the window on hangup. */
  async function hangup(): Promise<void> {
    if (session.closing) return;
    session.closing = true;
    setPhase("closing");
    try {
      await stopCall();
      await windowApi.close(WINDOW);
    } catch (error) {
      session.closing = false;
      setFailure(report(error));
      setPhase("error");
    }
  }

  ctx.useEffect("call-auto-start", () => {
    let disposed = false;
    if (autoStart) void (async () => {
      // A native child can render before its owner finishes registering the window.
      for (let attempt = 0; attempt < 100 && !disposed; attempt++) {
        const windows = await windowApi.list();
        if (disposed) return;
        if (windows.some(window => window.id === WINDOW)) {
          await run();
          return;
        }
        await new Promise<void>(resolve => setTimeout(resolve, 50));
      }
      if (!disposed) throw new Error(text("windowNotReady"));
    })().catch(error => {
      if (!disposed) { setFailure(report(error)); setPhase("error"); }
    });
    return () => { disposed = true; };
  }, []);

  const ui = ctx.UI;
  const colors = ctx.MaterialTheme.colorScheme;
  const canStart = phase === "idle" || phase === "error";
  const listening = phase === "listening";
  const speaking = phase === "speaking";
  const statusColor = phase === "error" ? colors.error : colors.primary;
  const statusIcon = listening ? "mic" : phase === "speaking" ? "graphic_eq" : canStart ? "call" : "more_horiz";
  const detail = failure || (phase === "speaking" ? reply : heard ? `${text("you")}  ${heard}` : "");
  const status = [
    ui.Row({ verticalAlignment: "center", spacing: 5, paddingHorizontal: 11, paddingVertical: 5,
      background: statusColor.copy({ alpha: 0.1 }), backgroundShape: { type: "rounded", radius: 20 },
    }, [
      ui.Icon({ name: statusIcon, size: 14, tint: statusColor }),
      ui.Text({ text: text(phase), style: "labelMedium", color: statusColor }),
    ]),
    ...(cameraNotice ? [ui.Text({ text: cameraNotice, textAlign: "center", style: "bodySmall",
      color: colors.onSurfaceVariant })] : []),
    ...(detail ? [ui.Text({ text: detail, fillMaxWidth: true, textAlign: "center", style: "bodySmall",
      color: failure ? colors.error : colors.onSurfaceVariant })] : []),
  ];
  const actions = ui.Row({ spacing: 10, verticalAlignment: "center", fillMaxWidth: true, horizontalArrangement: "center" }, [
    ...(canStart || listening || speaking ? [ui.FilledTonalButton({
      weight: 1,
      text: text(speaking ? "interrupt" : listening ? "submit" : session.chatId ? "retry" : "start"),
      onClick: speaking ? interruptSpeech : listening ? finishRecording : run,
    })] : []),
    ui.Button({ weight: 1, enabled: phase !== "closing", onClick: hangup,
      containerColor: colors.error, contentColor: colors.onError,
    }, [ui.Row({ spacing: 6, verticalAlignment: "center" }, [
      ui.Icon({ name: "call_end", size: 16, tint: colors.onError }),
      ui.Text({ text: text("hangup"), style: "labelLarge", color: colors.onError }),
    ])]),
  ]);
  return ui.MediaStage({
    fillMaxSize: true,
    camera: cameraControls,
    imageUri: identity?.avatarUri ?? undefined,
    imageTitle: identity?.title,
    cameraReadyText: getLang().startsWith("zh") ? "每轮发言共享一张画面" : "Shares one frame per spoken turn",
  }, [
    ui.Column({ fillMaxWidth: true, horizontalAlignment: "center", spacing: 6 }, [
      ui.ScrollArea({ weight: 1, fillMaxWidth: true }, [
        ui.Column({ fillMaxWidth: true, horizontalAlignment: "center", spacing: 4 }, status),
      ]),
      actions,
    ]),
  ]);
}

/** Publishes one launcher, shared by the toolbox and chat attachment menu. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "control", route: CONTROL_ROUTE, runtime: "compose_dsl", screen, title });
  ToolPkg.registerNavigationEntry({ id: "input-menu", route: CONTROL_ROUTE, surface: "chat_attachments",
    title: { zh: "语音/视频通话", en: "Voice / video call" }, icon: "Call", order: 50 });
  ToolPkg.registerNavigationEntry({ id: "control", route: CONTROL_ROUTE, title, surface: "toolbox", icon: "Call" });
  return true;
}
