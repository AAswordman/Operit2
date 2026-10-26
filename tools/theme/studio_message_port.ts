/** Trusted Studio-side protocol. It does not parse ZIPs or execute theme code. */
export interface ThemePreviewAsset {
  path: string;
  mediaType: "image/png" | "image/jpeg" | "image/svg+xml" | "font/ttf";
  sha256: string;
  bytes: Uint8Array;
}
export interface PreviewViewport { width: number; height: number }
export interface ClientShellReceipt {
  scope: "client-shell";
  parityVerified: boolean;
  revision: number;
  fixtureVersion: string;
  showConversation: boolean;
  fixtureSourceSha256: string;
  themeSha256: string;
  brightness: "light" | "dark";
  viewport: PreviewViewport;
  fonts: { reference: string; sha256: string; family: string; bytes: number }[];
  assetSha256: Record<string, string>;
}
interface Header { protocolVersion: 1; nonce: string; rendererReleaseId: string }
export type ThemeStudioMessage =
  | ({ type: "operit.theme.preview.ready.v1"; nextRevision: number } & Header)
  | ({ type: "operit.theme.preview.progress.v1"; revision: number; stage: string } & Header)
  | ({ type: "operit.theme.preview.loaded.v1"; revision: number; themeSha256: string } & Header)
  | ({ type: "operit.theme.preview.rendered.v1"; revision: number; fixtureVersion: string; receipt: ClientShellReceipt } & Header)
  | ({ type: "operit.theme.preview.captured.v1"; revision: number; themeSha256: string; fixtureVersion: string; showConversation: boolean; viewport: PreviewViewport; pngSha256: string; pngBytes: Uint8Array } & Header)
  | ({ type: "operit.theme.preview.selection.v1"; revision: number; enabled: boolean; targetId: string | null } & Header)
  | ({ type: "operit.theme.preview.error.v1"; revision?: number; code: string; message: string } & Header);
export interface ThemeBinaryExportInput {
  metadataJson: Uint8Array;
  themeJson: Uint8Array;
  previewPng: Uint8Array;
  assets: readonly ThemePreviewAsset[];
}
export interface ThemeContractBridge {
  normalizeDraft(themeJson: Uint8Array): Promise<string>;
  validatePreview(themeJson: Uint8Array, assets: readonly ThemePreviewAsset[]): Promise<string>;
  exportPackage(input: ThemeBinaryExportInput): Promise<Uint8Array>;
}
export class ThemeStudioProtocolError extends Error {
  readonly code: string;
  constructor(code: string, message: string) { super(message); this.code = code; }
}
export interface ThemeStudioChannelOptions {
  iframe: HTMLIFrameElement;
  previewUrl: string;
  nonce: string;
  rendererReleaseId: string;
  fixtureVersion: string;
  fixtureSourceSha256: string;
}

/** Subscribe BEFORE navigating the iframe. Availability is advertised only
 * after the preview's own contract worker starts. "loaded" is not a frame. */
export class ThemeStudioMessagePort {
  private readonly options: ThemeStudioChannelOptions;
  private readonly previewUrl: URL;
  private readonly channel = new MessageChannel();
  private state: "idle" | "connecting" | "connected" | "failed" | "closed" = "idle";
  private connectTimer: ReturnType<typeof setTimeout> | undefined;
  private renderTimer: ReturnType<typeof setTimeout> | undefined;
  private handler: ((message: ThemeStudioMessage) => void) | undefined;
  private requestedRevision = 0;
  private expectedThemeSha256: string | undefined;
  private expectedAssets: Record<string, string> = Object.create(null);
  private expectedFonts: string[] = [];
  private renderedReceipt: ClientShellReceipt | null = null;
  private portTransferred = false;
  private expectedBrightness: "light" | "dark" = "light";
  private expectedShowConversation = false;
  private expectedViewport: PreviewViewport = { width: 1280, height: 800 };
  private capturePending: { revision: number; resolve: (bytes: Uint8Array) => void; reject: (error: unknown) => void; timer: ReturnType<typeof setTimeout> } | undefined;
  private inspectionEnabled = false;
  private readyPromise: Promise<void> | undefined;
  private resolveReady: (() => void) | undefined;
  private rejectReady: ((error: unknown) => void) | undefined;

  constructor(options: ThemeStudioChannelOptions) {
    this.options = { ...options };
    this.previewUrl = new URL(options.previewUrl);
    if (!["http:", "https:"].includes(this.previewUrl.protocol) ||
        this.previewUrl.username || this.previewUrl.password || this.previewUrl.hash ||
        !/^[A-Za-z0-9_-]{16,128}$/.test(options.nonce) ||
        !/^[A-Za-z0-9._-]{1,128}$/.test(options.rendererReleaseId) || !options.fixtureVersion ||
        !isSha256(options.fixtureSourceSha256)) {
      throw new ThemeStudioProtocolError("invalid-configuration", "an exact preview URL, nonce and pinned renderer/fixture are required");
    }
  }
  get receipt(): ClientShellReceipt | null {
    return this.renderedReceipt === null ? null : structuredClone(this.renderedReceipt);
  }
  get failed(): boolean { return this.state === "failed"; }
  get connected(): boolean { return this.state === "connected"; }
  waitUntilReady(): Promise<void> {
    if (this.state === "connected") return Promise.resolve();
    if (this.state === "failed" || this.state === "closed") {
      return Promise.reject(new ThemeStudioProtocolError("preview-unavailable", "preview channel is not available"));
    }
    if (this.readyPromise === undefined) {
      return Promise.reject(new ThemeStudioProtocolError("not-started", "preview channel has not started"));
    }
    return this.readyPromise;
  }
  connect(onMessage: (message: ThemeStudioMessage) => void): void {
    if (this.state !== "idle") throw new ThemeStudioProtocolError("already-connected", "a preview channel is single-use");
    this.state = "connecting";
    this.handler = onMessage;
    this.readyPromise = new Promise<void>((resolve, reject) => {
      this.resolveReady = resolve;
      this.rejectReady = reject;
    });
    // A caller that only subscribes to protocol messages must not create an
    // unhandled rejection. waitUntilReady() still observes the same promise.
    void this.readyPromise.catch(() => undefined);
    window.addEventListener("message", this.onWindowMessage);
    this.channel.port1.onmessage = (event) => this.onPortMessage(event.data);
    this.channel.port1.onmessageerror = () => this.fail("invalid-message", "preview response could not be cloned");
    this.channel.port1.start();
    this.connectTimer = setTimeout(() => this.fail("preview-startup-timeout", "preview did not finish its pinned contract startup"), 15_000);
    this.previewUrl.searchParams.set("studioOrigin", window.location.origin);
    this.previewUrl.searchParams.set("nonce", this.options.nonce);
    this.previewUrl.searchParams.set("rendererReleaseId", this.options.rendererReleaseId);
    this.options.iframe.src = this.previewUrl.href;
  }
  private readonly onWindowMessage = (event: MessageEvent): void => {
    if (this.state !== "connecting" || event.origin !== this.previewUrl.origin ||
        event.source !== this.options.iframe.contentWindow) return;
    const message: unknown = event.data;
    if (!isRecord(message) || message.nonce !== this.options.nonce) return;
    try {
      this.requireHeader(message);
      if (message.type === "operit.theme.preview.error.v1") {
        if (typeof message.code !== "string" || typeof message.message !== "string") throw new Error("invalid startup error");
        this.fail(message.code, message.message);
        return;
      }
      if (message.type !== "operit.theme.preview.available.v1" || this.portTransferred) return;
      const previewWindow = this.options.iframe.contentWindow;
      if (!previewWindow) throw new Error("preview iframe has no content window");
      this.portTransferred = true;
      previewWindow.postMessage({ type: "operit.theme.studio.connect.v1", ...this.header() },
        this.previewUrl.origin, [this.channel.port2]);
    } catch (error) { this.fail("invalid-handshake", String(error)); }
  };
  loadTheme(options: {
    revision: number; canonicalThemeJson: string; themeSha256: string;
    assets: readonly ThemePreviewAsset[]; brightness?: "light" | "dark";
    viewport?: PreviewViewport; showConversation: boolean;
  }): void {
    this.requireConnected();
    if (!Number.isSafeInteger(options.revision) || options.revision <= this.requestedRevision) {
      throw new ThemeStudioProtocolError("stale-revision", "preview revisions must increase");
    }
    // Invalid edits must not retain success evidence for a previous edit.
    this.requestedRevision = options.revision;
    this.inspectionEnabled = false;
    this.renderedReceipt = null;
    this.rejectCapture("superseded", "capture belongs to an older revision");
    clearTimeout(this.renderTimer);
    this.expectedThemeSha256 = undefined;
    this.expectedAssets = Object.create(null);
    this.expectedFonts = [];
    if (!isSha256(options.themeSha256) || new TextEncoder().encode(options.canonicalThemeJson).length > 256 * 1024 || options.assets.length > 24) {
      throw new ThemeStudioProtocolError("invalid-draft", "draft identity or transport budget is invalid");
    }
    if (options.brightness !== undefined && options.brightness !== "light" && options.brightness !== "dark") {
      throw new ThemeStudioProtocolError("invalid-draft", "preview brightness must be light or dark");
    }
    if (typeof options.showConversation !== "boolean") {
      throw new ThemeStudioProtocolError("invalid-draft", "conversation mode must be boolean");
    }
    const viewport = options.viewport ?? { width: 1280, height: 800 };
    if (!isViewport(viewport)) {
      throw new ThemeStudioProtocolError("invalid-draft", "preview viewport must use bounded integer dimensions");
    }
    this.expectedBrightness = options.brightness ?? "light";
    this.expectedShowConversation = options.showConversation;
    this.expectedViewport = { ...viewport };
    const theme: unknown = JSON.parse(options.canonicalThemeJson);
    if (!isRecord(theme) || !isRecord(theme.typography) || typeof theme.typography.global !== "string" ||
        (theme.typography.user !== null && typeof theme.typography.user !== "string") ||
        (theme.typography.ai !== null && typeof theme.typography.ai !== "string")) {
      throw new ThemeStudioProtocolError("invalid-draft", "normalized typography is required for frame identity");
    }
    const typography = theme.typography;
    this.expectedFonts = [typography.global as string, "cascadia-mono",
      typography.user === null ? typography.global as string : typography.user as string,
      typography.ai === null ? typography.global as string : typography.ai as string];
    const assets: ThemePreviewAsset[] = [];
    let totalBytes = 0;
    for (const asset of options.assets) {
      if (!(asset.bytes instanceof Uint8Array)) throw new ThemeStudioProtocolError("invalid-asset", "binary bytes are required");
      totalBytes += asset.bytes.byteLength;
      if (typeof asset.path !== "string" || !isSha256(asset.sha256) ||
          !asset.bytes.byteLength || asset.bytes.byteLength > 32 * 1024 * 1024 ||
          totalBytes > 128 * 1024 * 1024 || Object.hasOwn(this.expectedAssets, asset.path)) {
        throw new ThemeStudioProtocolError("invalid-asset", "binary assets must be bounded, unique and hash-bound");
      }
      assets.push({ ...asset, bytes: Uint8Array.from(asset.bytes) });
      this.expectedAssets[asset.path] = asset.sha256;
    }
    this.expectedThemeSha256 = options.themeSha256;
    this.post({ type: "operit.theme.preview.load.v1", ...options,
      brightness: this.expectedBrightness, showConversation: this.expectedShowConversation,
      viewport: this.expectedViewport, assets });
    // First Flutter Web frame may include pinned CJK/font and CanvasKit setup.
    // Keep the validator's 15 s task bound, but give the renderer its own budget.
    this.renderTimer = setTimeout(() => this.fail("preview-render-timeout", "the current draft has no completed frame"), 60_000);
  }
  setConversationMode(options: { revision: number; showConversation: boolean }): void {
    this.requireConnected();
    if (options.revision !== this.requestedRevision || !Number.isSafeInteger(options.revision)) {
      throw new ThemeStudioProtocolError("stale-revision", "conversation mode must bind to the current rendered revision");
    }
    if (typeof options.showConversation !== "boolean") {
      throw new ThemeStudioProtocolError("invalid-scene-mode", "conversation mode must be boolean");
    }
    if (this.inspectionEnabled) {
      throw new ThemeStudioProtocolError("inspection-active", "exit element selection before changing the preview scene");
    }
    if (this.renderedReceipt?.revision !== options.revision) {
      throw new ThemeStudioProtocolError("preview-not-rendered", "conversation mode requires the current rendered Flutter frame");
    }
    if (this.renderedReceipt.showConversation === options.showConversation) return;
    this.expectedShowConversation = options.showConversation;
    this.renderedReceipt = null;
    this.rejectCapture("superseded", "capture belongs to an older preview scene");
    clearTimeout(this.renderTimer);
    this.post({ type: "operit.theme.preview.sceneMode.v1", revision: options.revision,
      showConversation: options.showConversation });
    this.renderTimer = setTimeout(() => this.fail("preview-render-timeout", "the selected preview scene has no completed frame"), 60_000);
  }
  setInspectMode(options: { revision: number; enabled: boolean; targetId?: string | null }): void {
    this.requireConnected();
    if (options.revision !== this.requestedRevision || !Number.isSafeInteger(options.revision)) {
      throw new ThemeStudioProtocolError("stale-revision", "inspection mode must bind to the current rendered revision");
    }
    if (this.renderedReceipt?.revision !== options.revision) {
      throw new ThemeStudioProtocolError("preview-not-rendered", "inspection mode requires the current rendered Flutter frame");
    }
    const targetId = options.targetId ?? null;
    const allowedTargets = new Set([
      "background", "navigation.sidebar", "chat.topBar", "chat.userMessage", "chat.aiMessage", "chat.composer",
    ]);
    if (targetId !== null && !allowedTargets.has(targetId)) {
      throw new ThemeStudioProtocolError("invalid-target", "inspection target ID is invalid");
    }
    if (!options.enabled && targetId !== null) {
      throw new ThemeStudioProtocolError("invalid-target", "disabled inspection cannot retain a selected target");
    }
    this.inspectionEnabled = options.enabled;
    this.post({ type: "operit.theme.preview.inspectMode.v1", revision: options.revision, enabled: options.enabled, targetId });
  }

  requestReceipt(): void {
    this.requireConnected();
    if (this.expectedThemeSha256 === undefined) throw new ThemeStudioProtocolError("no-draft", "load a checked draft before requesting its frame");
    this.post({ type: "operit.theme.preview.requestReceipt.v1", revision: this.requestedRevision });
  }
  capturePng(): Promise<Uint8Array> {
    this.requireConnected();
    if (this.inspectionEnabled) {
      throw new ThemeStudioProtocolError("capture-inspection-active", "disable selection mode before capturing a clean Flutter frame");
    }
    if (this.capturePending !== undefined || this.renderedReceipt === null) {
      throw new ThemeStudioProtocolError("capture-not-ready", "the current rendered frame is not ready for capture");
    }
    const revision = this.requestedRevision;
    return new Promise<Uint8Array>((resolve, reject) => {
      const timer = setTimeout(() => this.rejectCapture("capture-timeout", "Flutter did not return a current PNG frame"), 15_000);
      this.capturePending = { revision, resolve, reject, timer };
      this.post({ type: "operit.theme.preview.capture.v1", revision, viewport: this.expectedViewport,
        showConversation: this.expectedShowConversation });
    });
  }
  private rejectCapture(code: string, message: string): void {
    const pending = this.capturePending;
    if (pending === undefined) return;
    this.capturePending = undefined;
    clearTimeout(pending.timer);
    pending.reject(new ThemeStudioProtocolError(code, message));
  }
  private async finishCapture(data: Record<string, unknown>): Promise<void> {
    const pending = this.capturePending;
    if (pending === undefined || data.revision !== pending.revision ||
        data.themeSha256 !== this.expectedThemeSha256 || data.fixtureVersion !== this.options.fixtureVersion ||
        data.showConversation !== this.expectedShowConversation ||
        !sameViewport(data.viewport, this.expectedViewport) ||
        this.renderedReceipt?.revision !== pending.revision || !(data.pngBytes instanceof Uint8Array) ||
        data.pngBytes.byteLength < 33 || data.pngBytes.byteLength > 4 * 1024 * 1024 || !isSha256(data.pngSha256)) {
      this.fail("invalid-capture", "capture does not belong to the current rendered frame");
      return;
    }
    const bytes = Uint8Array.from(data.pngBytes);
    const magic = [137, 80, 78, 71, 13, 10, 26, 10];
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    if (magic.some((byte, index) => bytes[index] !== byte) ||
        view.getUint32(16) !== this.expectedViewport.width ||
        view.getUint32(20) !== this.expectedViewport.height ||
        await sha256(bytes) !== data.pngSha256) {
      this.fail("invalid-capture", "capture PNG bytes or digest are invalid");
      return;
    }
    if (this.capturePending !== pending || this.renderedReceipt?.revision !== pending.revision) return;
    this.capturePending = undefined;
    clearTimeout(pending.timer);
    pending.resolve(bytes);
  }
  private onPortMessage(data: unknown): void {
    if (this.state === "closed" || this.state === "failed") return;
    if (!isRecord(data) || data.nonce !== this.options.nonce) return;
    try {
      this.requireHeader(data);
      if (data.type === "operit.theme.preview.ready.v1") {
        if (this.state !== "connecting" || !this.portTransferred || !Number.isSafeInteger(data.nextRevision) || (data.nextRevision as number) < 1) throw new Error("invalid ready revision");
        this.requestedRevision = (data.nextRevision as number) - 1;
        this.state = "connected";
        clearTimeout(this.connectTimer);
        window.removeEventListener("message", this.onWindowMessage);
        this.resolveReady?.();
        this.resolveReady = undefined;
        this.rejectReady = undefined;
      } else {
        this.requireConnected();
        if (!Number.isSafeInteger(data.revision)) throw new Error("response needs a revision");
        if ((data.revision as number) < this.requestedRevision) return;
        if (data.revision !== this.requestedRevision) throw new Error("response belongs to an unrequested revision");
        switch (data.type) {
          case "operit.theme.preview.selection.v1":
            if (typeof data.enabled !== "boolean" || data.enabled !== this.inspectionEnabled ||
                (data.targetId !== null && (typeof data.targetId !== "string" ||
                  !["background", "navigation.sidebar", "chat.topBar", "chat.userMessage", "chat.aiMessage", "chat.composer"].includes(data.targetId))) ||
                (!data.enabled && data.targetId !== null)) {
              throw new Error("selection response is invalid");
            }
            break;
          case "operit.theme.preview.progress.v1":
            if (typeof data.stage !== "string" || !/^[a-z-]{1,40}$/.test(data.stage)) throw new Error("invalid preview stage");
            break;
          case "operit.theme.preview.loaded.v1":
            if (data.themeSha256 !== this.expectedThemeSha256 || this.expectedThemeSha256 === undefined) throw new Error("loaded theme hash mismatch");
            break;
          case "operit.theme.preview.rendered.v1":
            this.renderedReceipt = structuredClone(this.checkReceipt(data));
            clearTimeout(this.renderTimer);
            break;
          case "operit.theme.preview.captured.v1":
            void this.finishCapture(data).catch(error => this.fail("invalid-capture", String(error)));
            return;
          case "operit.theme.preview.error.v1":
            if (typeof data.code !== "string" || typeof data.message !== "string") throw new Error("invalid preview error");
            this.renderedReceipt = null;
            this.rejectCapture(data.code as string, data.message as string);
            clearTimeout(this.renderTimer);
            break;
          default: throw new Error("unregistered preview message");
        }
      }
      this.handler?.(structuredClone(data) as unknown as ThemeStudioMessage);
    } catch (error) { this.fail("invalid-preview-response", String(error)); }
  }
  private checkReceipt(message: Record<string, unknown>): ClientShellReceipt {
    const receipt = message.receipt;
    if (!isRecord(receipt) || receipt.scope !== "client-shell" ||
        typeof receipt.parityVerified !== "boolean" ||
        receipt.revision !== this.requestedRevision || this.expectedThemeSha256 === undefined ||
        receipt.themeSha256 !== this.expectedThemeSha256 ||
        receipt.brightness !== this.expectedBrightness ||
        receipt.showConversation !== this.expectedShowConversation ||
        !sameViewport(receipt.viewport, this.expectedViewport) ||
        message.fixtureVersion !== this.options.fixtureVersion || receipt.fixtureVersion !== this.options.fixtureVersion ||
        receipt.fixtureSourceSha256 !== this.options.fixtureSourceSha256 ||
        !Array.isArray(receipt.fonts) || receipt.fonts.length !== 4 ||
        !isRecord(receipt.assetSha256)) throw new Error("frame receipt is incomplete or belongs to another draft");
    const assetDigests = receipt.assetSha256;
    if (Object.keys(assetDigests).length !== Object.keys(this.expectedAssets).length ||
        Object.entries(this.expectedAssets).some(([path, hash]) => assetDigests[path] !== hash)) throw new Error("frame asset hashes mismatch");
    for (const [index, font] of receipt.fonts.entries()) {
      if (!isRecord(font) || font.reference !== this.expectedFonts[index] || !isSha256(font.sha256) ||
          font.family !== "OperitThemeFont_" + font.sha256 || !Number.isSafeInteger(font.bytes) || (font.bytes as number) <= 0 ||
          (font.bytes as number) > 32 * 1024 * 1024 ||
          (Object.hasOwn(this.expectedAssets, this.expectedFonts[index]) && this.expectedAssets[this.expectedFonts[index]] !== font.sha256)) throw new Error("frame font receipt is invalid");
    }
    return receipt as unknown as ClientShellReceipt;
  }
  private requireHeader(message: Record<string, unknown>): void {
    if (message.protocolVersion !== 1 || message.rendererReleaseId !== this.options.rendererReleaseId) throw new Error("protocol or renderer release mismatch");
  }
  private requireConnected(): void {
    if (this.state !== "connected") throw new ThemeStudioProtocolError("not-connected", "preview has not finished its contract handshake");
  }
  private header(): Header { return { protocolVersion: 1, nonce: this.options.nonce, rendererReleaseId: this.options.rendererReleaseId }; }
  private post(message: Record<string, unknown>): void { this.channel.port1.postMessage({ ...message, ...this.header() }); }
  private fail(code: string, message: string): void {
    if (this.state === "failed" || this.state === "closed") return;
    this.state = "failed";
    this.renderedReceipt = null;
    this.rejectCapture(code, message);
    this.rejectReady?.(new ThemeStudioProtocolError(code, message));
    this.resolveReady = undefined;
    this.rejectReady = undefined;
    this.detach();
    this.handler?.({ type: "operit.theme.preview.error.v1", ...this.header(), revision: this.requestedRevision, code, message });
  }
  private detach(): void {
    clearTimeout(this.connectTimer);
    clearTimeout(this.renderTimer);
    window.removeEventListener("message", this.onWindowMessage);
    this.channel.port1.close();
    this.channel.port2.close();
  }
  close(): void {
    if (this.state === "closed") return;
    this.detach();
    this.state = "closed";
    this.renderedReceipt = null;
    this.rejectCapture("closed", "capture channel was closed");
    this.rejectReady?.(new ThemeStudioProtocolError("closed", "preview channel was closed"));
    this.resolveReady = undefined;
    this.rejectReady = undefined;
    this.handler = undefined;
  }
}

/** Raw contract export. A client-shell receipt is NOT a cross-platform 1:1 screenshot or
 * whole-scene parity/export authorization; the final UI needs both. */
export async function exportWithContract(bridge: ThemeContractBridge, input: ThemeBinaryExportInput): Promise<Uint8Array> {
  const response: unknown = JSON.parse(await bridge.normalizeDraft(input.themeJson));
  if (!isRecord(response) || response.ok !== true) {
    throw new ThemeStudioProtocolError("draft-invalid", JSON.stringify(response));
  }
  return bridge.exportPackage(input);
}
function isRecord(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === "object" && !Array.isArray(value); }
function isSha256(value: unknown): value is string { return typeof value === "string" && /^[a-f0-9]{64}$/.test(value); }
function isViewport(value: unknown): value is PreviewViewport {
  return isRecord(value) && typeof value.width === "number" && typeof value.height === "number" &&
    Number.isSafeInteger(value.width) && Number.isSafeInteger(value.height) &&
    value.width >= 240 && value.width <= 2400 && value.height >= 360 && value.height <= 2400;
}
function sameViewport(value: unknown, expected: PreviewViewport): boolean {
  return isViewport(value) && value.width === expected.width && value.height === expected.height;
}
async function sha256(bytes: Uint8Array): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", Uint8Array.from(bytes).buffer as ArrayBuffer);
  return Array.from(new Uint8Array(hash), byte => byte.toString(16).padStart(2, "0")).join("");
}
