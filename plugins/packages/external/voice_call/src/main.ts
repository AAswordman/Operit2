import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Chat } from "../../../../types/chat";
import { characterCards, type ChatDisplayIdentity } from "./dependencies/character_cards/api.js";

const PACKAGE = "com.operit.voice_call";
const CONTROL_ROUTE = `toolpkg:${PACKAGE}:ui:control`;
const MENU_ROUTE = `toolpkg:${PACKAGE}:ui:input-menu`;
const CALL_ROUTE = `toolpkg:${PACKAGE}:ui:call`;
const VOICE_OVERLAY_ROUTE = `toolpkg:${PACKAGE}:ui:voice-overlay`;
const WINDOW = "voice-call";
const title = { zh: "语音通话", en: "Voice Call" };
const RECORDING_OPTIONS = { maxDurationMs: 20000, silenceMs: 900 };
const INTERRUPT_LEVEL = 0.35;
const INTERRUPT_SAMPLES = 2;

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

/** Launches a call from the chat attachment menu and settles its presentation. */
export function inputMenu(ctx: ComposeDslContext): ComposeNode {
  const [chatId] = ctx.useState<string | null>("chatId", null);
  const [presentation] = ctx.useState<{ requestId: string } | null>("presentation", null);
  if (!presentation || typeof presentation.requestId !== "string" || !presentation.requestId.trim()) {
    throw new Error("Missing call launcher presentation request");
  }
  const requestId = presentation.requestId;
  const [error, setError] = ctx.useState("call-launch-error", "");
  const closed = ctx.useRef("call-launch-closed", false);
  const busy = ctx.useRef("call-launch-busy", false);
  const zh = getLang().startsWith("zh");
  function close(): ToolPkg.JsonValue {
    closed.current = true;
    return { type: "toolpkg.presentation.cancel", requestId };
  }
  async function open(): Promise<ToolPkg.JsonValue> {
    if (busy.current || closed.current) return null;
    busy.current = true;
    setError("");
    try {
      const api = ToolPkg.ui.floatingWindows;
      let caps = await api.capabilities();
      const applicationOnly = caps.cameraRequiresApplicationWindow === true;
      if (applicationOnly) caps = await api.capabilities({ applicationOnly });
      if (closed.current) return null;
      if (!caps.supported) throw new Error(text("unsupported"));
      if (!caps.permissionGranted && !await api.requestPermission()) throw new Error(text("permission"));
      if (closed.current) return null;
      const existing = (await api.list()).find(window => window.id === WINDOW);
      if (closed.current) return null;
      if (existing && existing.route !== CALL_ROUTE) await api.close(WINDOW);
      if (closed.current) return null;
      if (existing?.route === CALL_ROUTE) {
        await api.show(WINDOW);
        await api.setFullscreen(WINDOW, true);
      } else {
        await api.open({ id: WINDOW, route: CALL_ROUTE, fill: true, applicationOnly,
          state: { autoStart: true, sourceChatId: chatId },
        });
      }
      return close();
    } catch (failure) {
      if (!closed.current) setError(report(failure));
      return null;
    } finally { busy.current = false; }
  }
  return ctx.UI.AlertDialog({
    key: "voice-call-launcher", onLoad: open,
    title: zh ? "语音/视频通话" : "Voice / video call",
    text: error || (zh ? "正在打开通话…" : "Opening call…"),
    confirmText: zh ? "重试" : "Retry", onConfirm: open, closeOnConfirm: false,
    dismissText: zh ? "关闭" : "Close", onDismiss: close, onDismissRequest: close,
  });
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
          state: { cameraControls: !voiceOverlay },
          route: voiceOverlay ? VOICE_OVERLAY_ROUTE : CALL_ROUTE,
          options: { width: 340, height: voiceOverlay ? 404 : 460, draggable: true },
        });
      }
      const system = caps.scope === "system" || (existing ? existing.route === VOICE_OVERLAY_ROUTE : voiceOverlay);
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
  }).current;

  async function interruptSpeech(): Promise<void> {
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
    await Promise.all([
      ...(session.sending ? [Tools.Chat.cancel(session.chatId).catch(report)] : []),
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

  async function record(echoCancellation = false): Promise<ToolPkg.FloatingWindowRecording | null> {
    session.recording = true;
    try {
      return await windowApi.record(WINDOW, { ...RECORDING_OPTIONS, ...(echoCancellation ? { echoCancellation } : {}) });
    } finally {
      session.recording = false;
    }
  }

  /** Listens during playback and carries the captured interruption into the next turn. */
  async function listenDuringPlayback(playback: Playback, echoCancellation: boolean): Promise<ToolPkg.FloatingWindowRecording | null> {
    let checking = false;
    let loudSamples = 0;
    const detector = setInterval(() => {
      if (checking || !session.active || playback.done || !session.recording || playback.cancellation) return;
      checking = true;
      void windowApi.recordingLevel(WINDOW).then(async level => {
        if (!session.active || session.playback !== playback || playback.done) return;
        loudSamples = Number.isFinite(level) && level >= INTERRUPT_LEVEL ? loudSamples + 1 : 0;
        if (loudSamples >= INTERRUPT_SAMPLES) await interruptSpeech();
      }).catch(error => {
        clearInterval(detector);
        report(error);
      }).finally(() => { checking = false; });
    }, 100);
    try {
      // The host bounds each capture. Keep listening if that limit expires during a long reply.
      while (session.active && !playback.failed) {
        const startedDuringPlayback = !playback.done;
        loudSamples = 0;
        const captured = await record(echoCancellation);
        if (!session.active || playback.failed) return null;
        if (captured) {
          if (!playback.done && !playback.cancellation && loudSamples < INTERRUPT_SAMPLES) continue;
          await interruptSpeech();
          return captured;
        }
        if (!startedDuringPlayback || playback.done) break;
      }
      return null;
    } finally {
      clearInterval(detector);
    }
  }

  async function speak(answer: string, sentAt: number, echoCancellation: boolean): Promise<ToolPkg.FloatingWindowRecording | null> {
    setPhase("speaking");
    const playback: Playback = { id: `call-${session.chatId}-${sentAt}`, done: false, failed: false };
    session.playback = playback;
    const speaking = Tools.Tts.speak({
      text: answer,
      interrupt: false,
      waitForCompletion: true,
      playbackId: playback.id,
    }).catch(async error => {
      if (await playback.cancellation) return;
      playback.failed = true;
      throw error;
    }).finally(() => {
      playback.done = true;
      if (session.active && !playback.failed && session.recording) setPhase("listening");
    });
    const listening = listenDuringPlayback(playback, echoCancellation).catch(error => {
      report(error);
      return null;
    });
    try {
      const [, captured] = await Promise.all([speaking, listening]);
      return captured;
    } catch (error) {
      if (session.recording) await windowApi.cancelRecording(WINDOW).catch(report);
      await listening;
      throw error;
    } finally {
      session.playback = null;
    }
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
      const caps = await cameraWindowCapabilities();
      const echoCancellation = caps.recordingEchoCancellation === true;
      let nextRecording: ToolPkg.FloatingWindowRecording | null = null;
      while (session.active) {
        if (!nextRecording) setPhase("listening");
        const recording = nextRecording ?? await record();
        nextRecording = null;
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
        nextRecording = await speak(answer, turn.sentAt, echoCancellation);
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

/** Registers the toolbox launcher and the standalone call surface. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "control", route: CONTROL_ROUTE, runtime: "compose_dsl", screen: control, title });
  ToolPkg.registerUiRoute({ id: "call", route: CALL_ROUTE, runtime: "compose_dsl", screen: call, title });
  ToolPkg.registerUiRoute({ id: "voice-overlay", route: VOICE_OVERLAY_ROUTE, runtime: "compose_dsl", screen: call, title });
  ToolPkg.registerUiRoute({ id: "input-menu", route: MENU_ROUTE, runtime: "compose_dsl", screen: inputMenu, title });
  ToolPkg.registerNavigationEntry({ id: "input-menu", route: MENU_ROUTE, surface: "chat_attachments",
    title: { zh: "语音/视频通话", en: "Voice / video call" }, icon: "Call", order: 50 });
  ToolPkg.registerNavigationEntry({ id: "control", route: CONTROL_ROUTE, title, surface: "toolbox", icon: "Call" });
  return true;
}
