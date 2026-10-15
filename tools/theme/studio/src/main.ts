import "./main.css";
import {
  ThemeStudioMessagePort,
  exportWithContract,
  type ClientShellReceipt,
  type ThemePreviewAsset,
  type ThemeStudioMessage,
  type PreviewViewport,
} from "../../studio_message_port.ts";
import { StudioContractBridge } from "../../studio_contract_bridge.ts";
import { loadDraft, saveDraft, type SavedAsset } from "./draft_store";
import { parseImportedThemeFile, updateToolPkgTheme, validateToolPkgId, type ImportedToolPkgSource, type ToolPkgThemeDeclaration } from "./toolpkg_import";
import { COMPONENT_COLOR_MAP, THEME_INSPECTOR_TARGETS, ThemeVisualEditor, defaultTheme } from "./theme_editor";

const SHA256 = /^[a-f0-9]{64}$/;
const METADATA_IDS = [
  "theme-id",
  "version",
  "display-name",
  "author",
  "description",
  "theme-license-text",
] as const;

const CANVAS_INSPECT_TARGET_IDS = new Set([
  "background",
  "navigation.sidebar",
  "chat.topBar",
  "chat.userMessage",
  "chat.aiMessage",
  "chat.composer",
]);

type DockTabId = "inspector" | "assets" | "manifest" | "json" | "all";
type EvidenceTabId = "receipt" | "messages" | "renderer";

type AssetRow = {
  id: number;
  path: string;
  file: File | null;
  bytes: Uint8Array | null;
  sha256: string | null;
  mediaType: ThemePreviewAsset["mediaType"] | null;
};

type RendererConfig = {
  ready: boolean;
  error?: string;
  rendererReleaseId: string;
  previewUrl: string;
  workerUrl: string;
  fixtureVersion: string;
  fixtureSourceSha256: string;
  rendererCommit: string;
  flutterRevision: string;
  sourceDirty: boolean;
};

const PREVIEW_VIEWPORTS: ReadonlyArray<{ id: string; label: string; viewport: PreviewViewport }> = [
  { id: "desktop", label: "桌面 1280×800", viewport: { width: 1280, height: 800 } },
  { id: "phone", label: "手机 390×844", viewport: { width: 390, height: 844 } },
  { id: "tablet", label: "平板 768×1024", viewport: { width: 768, height: 1024 } },
  { id: "wide", label: "宽屏 1440×900", viewport: { width: 1440, height: 900 } },
];

const LAYER_CHIPS: ReadonlyArray<{ id: string | null; label: string; shortKey: string }> = [
  { id: null, label: "全部", shortKey: "ALL" },
  { id: "background", label: "画布背景", shortKey: "BG" },
  { id: "navigation.sidebar", label: "导航侧栏", shortKey: "NAV" },
  { id: "chat.topBar", label: "对话顶栏", shortKey: "TOP" },
  { id: "chat.userMessage", label: "用户气泡", shortKey: "USER" },
  { id: "chat.aiMessage", label: "AI 气泡", shortKey: "AI" },
  { id: "chat.composer", label: "输入框", shortKey: "INPUT" },
  { id: "typography", label: "字体排版", shortKey: "FONT" },
  { id: "palette", label: "配色", shortKey: "PAL" },
];

class ThemeStudioApp {
  private readonly root: HTMLElement;
  private readonly status = document.createElement("output");
  private readonly messages = document.createElement("pre");
  private readonly receipt = document.createElement("pre");
  private readonly connectionState = document.createElement("span");
  private readonly themeJson = document.createElement("textarea");
  private readonly preview = document.createElement("iframe");
  private readonly previewCanvas = document.createElement("div");
  private readonly previewInspectOverlay = document.createElement("div");
  private readonly assetList = document.createElement("div");
  private readonly rendererUrl = document.createElement("input");
  private readonly workerUrl = document.createElement("input");
  private readonly rendererRelease = document.createElement("input");
  private readonly undoButton = document.createElement("button");
  private readonly redoButton = document.createElement("button");
  private readonly resetDefaultButton = document.createElement("button");
  private readonly importButton = document.createElement("button");
  private readonly saveButton = document.createElement("button");
  private readonly loadButton = document.createElement("button");
  private readonly renderButton = document.createElement("button");
  private readonly exportButton = document.createElement("button");
  private readonly captureButton = document.createElement("button");
  private readonly inspectButton = document.createElement("button");
  private readonly downloadPreviewButton = document.createElement("button");
  private readonly lightButton = document.createElement("button");
  private readonly darkButton = document.createElement("button");
  private readonly zoomButton = document.createElement("button");
  private readonly hudToggleButton = document.createElement("button");
  private readonly stageColorHud = document.createElement("div");
  private stageHudOpen = false;
  private autoInspectMode = true;
  private readonly viewportSelect = document.createElement("select");
  private readonly conversationToggle = document.createElement("input");
  private readonly addAssetButton = document.createElement("button");
  private readonly cleanUnusedButton = document.createElement("button");
  private readonly layerChipBar = document.createElement("div");
  private readonly dockTabBar = document.createElement("div");
  private readonly assetCountBadge = document.createElement("span");
  private readonly stageMetaStrip = document.createElement("div");
  private readonly liveThemeBadge = document.createElement("span");
  private readonly visualEditor: ThemeVisualEditor;
  private readonly rows: AssetRow[] = [];
  private readonly assetObjectUrls = new Map<string, string>();
  private rendererConfig: RendererConfig | null = null;
  private rendererConfigPromise: Promise<void>;
  private contract: StudioContractBridge | null = null;
  private channel: ThemeStudioMessagePort | null = null;
  private currentRevision = 0;
  private currentThemeSha256: string | null = null;
  private currentReceipt: ClientShellReceipt | null = null;
  private latestAssets: ThemePreviewAsset[] = [];
  private previewPng: Uint8Array | null = null;
  private nextAssetId = 1;
  private importedToolPkg: ImportedToolPkgSource | undefined;
  private busy = false;
  private iframeRendering = false;
  private generation = 0;
  private renderedGeneration = -1;
  private brightness: "light" | "dark" = "light";
  private showConversation = true;
  private originalPixels = false;
  private inspectEnabled = false;
  private inspectAfterRender = true;
  private selectedInspectTargetId: string | null = "background";
  private activeDockTab: DockTabId = "inspector";
  private activeEvidenceTab: EvidenceTabId = "receipt";
  private assetUploadTarget: string[] | null = null;
  private previewDebounceTimer: ReturnType<typeof setTimeout> | undefined;
  private historyDebounceTimer: ReturnType<typeof setTimeout> | undefined;
  private autoSaveTimer: ReturnType<typeof setTimeout> | undefined;
  private previewQueued = false;
  private viewport: PreviewViewport = { width: 1280, height: 800 };
  private historyStack: string[] = [];
  private historyIndex = -1;
  private applyingHistory = false;

  constructor(root: HTMLElement) {
    this.root = root;
    const visualHost = document.createElement("div");
    this.visualEditor = new ThemeVisualEditor(
      visualHost,
      () => this.syncVisualJson(),
      path => this.requestAssetUpload(path),
      () => this.selectInspectorLayer(null),
      path => this.getAssetPreviewUrl(path),
    );
    this.visualEditor.setBrightnessRequestHandler(mode => {
      if (this.brightness !== mode) {
        if (this.inspectEnabled && this.channel?.connected && this.currentReceipt?.revision === this.currentRevision) {
          try {
            this.channel.setInspectMode({ revision: this.currentRevision, enabled: false });
            this.inspectAfterRender = true;
          } catch {
            // Ignore
          }
          this.inspectEnabled = false;
        }
        this.brightness = mode;
        this.visualEditor.setPaletteMode(mode);
        this.updateBrightnessButtons();
        this.updateStageMetaStrip();
      }
    });
    this.visualEditor.setExternalDomSyncHandler(() => {
      this.renderStageColorHud();
    });

    this.root.innerHTML = `
      <header class="studio-header">
        <div class="brand-cluster">
          
          <div class="brand-copy">
            <div class="brand-title-row">
              <h1>Theme Studio</h1>
            </div>
          </div>
        </div>
        <div class="dock-tabs-slot"></div>
        <div class="header-actions-slot"></div>
      </header>
      <div class="status-ribbon">
        <div class="status-ribbon-left">
          <span class="status-dot" aria-hidden="true"></span>
          <output id="status"></output>
        </div>
        <div class="status-ribbon-right">
          <span id="live-theme-badge" class="live-theme-badge">ToolPkg</span>
          
          <span id="connection-state" class="state">starting</span>
        </div>
      </div>
      <section class="studio-grid">
        <form class="editor panel" id="editor-form"></form>
        <section class="preview-panel panel">
          <div class="stage-top-bar">
            <div class="preview-actions"></div>
          </div>
          <div class="stage-frame-shell">
            <div class="preview-viewport"><iframe title="Operit2 Flutter theme preview" referrerpolicy="no-referrer"></iframe></div>
          </div>
          <div class="stage-color-hud-slot"></div>
          <details class="log-panel evidence-drawer">
            <summary class="evidence-summary">
              <div class="evidence-summary-left">
                
                <span class="evidence-summary-title">诊断</span>
              </div>
              <div class="evidence-summary-right">
                <div class="stage-meta-slot"></div>
                
              </div>
            </summary>
            <div class="evidence-drawer-body">
              <div class="evidence-tabs" role="tablist">
                <button type="button" class="evidence-tab active" data-evidence-tab="receipt">渲染记录</button>
                <button type="button" class="evidence-tab" data-evidence-tab="messages">消息</button>
                <button type="button" class="evidence-tab" data-evidence-tab="renderer">构建信息</button>
              </div>
              <div class="evidence-panes">
                <div class="evidence-pane active" data-evidence-pane="receipt">
                  <pre id="receipt">尚未收到与当前草稿 SHA-256 绑定的 Flutter 渲染回执。</pre>
                </div>
                <div class="evidence-pane" data-evidence-pane="messages">
                  <pre id="messages">尚未建立预览通道。</pre>
                </div>
                <div class="evidence-pane" data-evidence-pane="renderer">
                  <div class="renderer-fields-host"></div>
                </div>
              </div>
            </div>
          </details>
        </section>
      </section>`;

    this.status.id = "status";
    this.messages.id = "messages";
    this.receipt.id = "receipt";
    this.connectionState.id = "connection-state";
    this.connectionState.className = "state";
    this.connectionState.textContent = "starting";
    this.liveThemeBadge.id = "live-theme-badge";
    this.liveThemeBadge.className = "live-theme-badge";

    this.root.querySelector("#status")!.replaceWith(this.status);
    this.root.querySelector("#messages")!.replaceWith(this.messages);
    this.root.querySelector("#receipt")!.replaceWith(this.receipt);
    this.root.querySelector("#connection-state")!.replaceWith(this.connectionState);
    this.root.querySelector("#live-theme-badge")!.replaceWith(this.liveThemeBadge);
    this.root.querySelector(".stage-meta-slot")!.replaceWith(this.stageMetaStrip);
    this.stageColorHud.className = "stage-color-hud hidden";
    this.root.querySelector(".stage-color-hud-slot")!.replaceWith(this.stageColorHud);

    const oldIframe = this.root.querySelector("iframe")!;
    this.preview.title = oldIframe.title;
    this.preview.referrerPolicy = "no-referrer";
    this.previewCanvas.className = "preview-canvas";
    this.previewInspectOverlay.className = "preview-inspect-overlay";
    this.previewCanvas.append(this.preview, this.previewInspectOverlay);
    oldIframe.replaceWith(this.previewCanvas);

    this.buildEditor(this.root.querySelector<HTMLElement>("#editor-form")!, visualHost);
    this.bindEvidenceTabs();
    this.updateStageMetaStrip();
    this.updateLiveThemeBadge();
    this.renderStageColorHud();
    this.pushHistorySnapshot(this.themeJson.value);
    this.resizePreview();
    const stageShell = this.root.querySelector<HTMLElement>(".stage-frame-shell")!;
    new ResizeObserver(() => this.resizePreview()).observe(stageShell);
    this.bindActions();
    this.selectInspectorLayer("background");
    this.bindGlobalShortcutsAndDrop();
    this.setStatus("正在读取本地 renderer 配置…");
    this.rendererConfigPromise = this.initialize();
    void this.rendererConfigPromise.then(() => {
      if (this.rendererConfig != null) void this.renderDraft();
    });
  }

  private pushHistorySnapshot(jsonText: string): void {
    if (this.applyingHistory) return;
    if (this.historyIndex >= 0 && this.historyStack[this.historyIndex] === jsonText) {
      this.updateHistoryButtons();
      return;
    }
    this.historyStack = this.historyStack.slice(0, this.historyIndex + 1);
    this.historyStack.push(jsonText);
    if (this.historyStack.length > 50) {
      this.historyStack.shift();
    } else {
      this.historyIndex += 1;
    }
    this.updateHistoryButtons();
  }

  private scheduleHistorySnapshot(): void {
    clearTimeout(this.historyDebounceTimer);
    this.historyDebounceTimer = setTimeout(() => {
      this.pushHistorySnapshot(this.themeJson.value);
    }, 280);
  }

  private updateHistoryButtons(): void {
    this.undoButton.disabled = this.historyIndex <= 0;
    this.redoButton.disabled = this.historyIndex < 0 || this.historyIndex >= this.historyStack.length - 1;
  }

  private undo(): void {
    if (this.historyIndex <= 0) return;
    this.applyingHistory = true;
    try {
      this.historyIndex -= 1;
      const snap = this.historyStack[this.historyIndex];
      this.visualEditor.setJson(snap, false);
      this.themeJson.value = this.visualEditor.json;
      this.updateHistoryButtons();
      this.renderAssetRows();
      this.scheduleAtomicAutoSave();
      this.invalidate("已撤销上一步参数修改；正在同步预览。");
      this.schedulePreview();
    } finally {
      this.applyingHistory = false;
    }
  }

  private redo(): void {
    if (this.historyIndex >= this.historyStack.length - 1) return;
    this.applyingHistory = true;
    try {
      this.historyIndex += 1;
      const snap = this.historyStack[this.historyIndex];
      this.visualEditor.setJson(snap, false);
      this.themeJson.value = this.visualEditor.json;
      this.updateHistoryButtons();
      this.renderAssetRows();
      this.scheduleAtomicAutoSave();
      this.invalidate("已重做参数修改；正在同步预览。");
      this.schedulePreview();
    } finally {
      this.applyingHistory = false;
    }
  }

  private resetToDefaultTheme(): void {
    const cleanJson = JSON.stringify(defaultTheme(), null, 2);
    this.visualEditor.setJson(cleanJson, false);
    this.themeJson.value = this.visualEditor.json;
    this.pushHistorySnapshot(this.themeJson.value);
    this.renderAssetRows();
    this.scheduleAtomicAutoSave();
    this.invalidate("已重置为官方默认主题参数；正在重新渲染预览。");
    void this.renderDraft();
  }

  private bindGlobalShortcutsAndDrop(): void {
    window.addEventListener("keydown", event => {
      const target = event.target as HTMLElement | null;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target?.isContentEditable
      ) {
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        if (event.shiftKey) {
          this.redo();
        } else {
          this.undo();
        }
      } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
        event.preventDefault();
        this.redo();
      }
    });

    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible" && this.rendererConfig != null) {
        if (this.channel == null || this.channel.failed || (this.currentReceipt == null && !this.busy)) {
          void this.renderDraft();
        }
      }
    });

    window.addEventListener("dragover", event => {
      event.preventDefault();
    });

    window.addEventListener("drop", event => {
      const file = event.dataTransfer?.files?.[0];
      if (!file) return;
      const lower = file.name.toLowerCase();
      if (lower.endsWith(".toolpkg") || lower.endsWith(".json")) {
        event.preventDefault();
        void this.importToolPkgTemplateFile(file);
      }
    });
  }

  private buildEditor(form: HTMLElement, visualHost: HTMLElement): void {
    this.dockTabBar.className = "dock-tab-bar";
    const tabs: Array<{ id: DockTabId; label: string; badge?: HTMLElement }> = [
      { id: "inspector", label: "设计" },
      { id: "assets", label: "素材", badge: this.assetCountBadge },
      { id: "manifest", label: "信息" },
      { id: "json", label: "JSON" },
      
    ];
    this.assetCountBadge.className = "dock-tab-count";
    this.assetCountBadge.textContent = "0";

    for (const tab of tabs) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = `dock-tab-btn${this.activeDockTab === tab.id ? " active" : ""}`;
      btn.dataset.dockTab = tab.id;
      const titleSpan = document.createElement("span");
      titleSpan.className = "dock-tab-label";
      titleSpan.textContent = tab.label;
      btn.append(titleSpan);
      if (tab.badge != null) {
        btn.append(tab.badge);
      }
      btn.onclick = () => this.switchDockTab(tab.id);
      this.dockTabBar.append(btn);
    }
    this.root.querySelector(".dock-tabs-slot")!.replaceWith(this.dockTabBar);

    this.layerChipBar.className = "component-layer-bar";
    this.renderLayerChips();

    const visual = document.createElement("section");
    visual.className = "visual-section dock-section";
    visual.dataset.section = "inspector";
    visualHost.id = "visual-editor";
    visual.append(this.layerChipBar, visualHost);
    form.append(visual);

    const assets = document.createElement("section");
    assets.className = "assets-section dock-section";
    assets.dataset.section = "assets";
    assets.innerHTML = `
      <div class="section-heading">
        <div class="section-heading-top">
          <h3>素材</h3>
          <span class="section-pill">PNG · JPEG · SVG · TTF</span>
        </div>
        
      </div>`;
    const assetToolbar = document.createElement("div");
    assetToolbar.className = "asset-toolbar-row";
    assetToolbar.append(this.addAssetButton, this.cleanUnusedButton);
    this.assetList.className = "asset-list";
    assets.append(assetToolbar, this.assetList);
    form.append(assets);

    const metadata = document.createElement("section");
    metadata.className = "metadata-section dock-section";
    metadata.dataset.section = "manifest";
    metadata.innerHTML = `
      <div class="section-heading">
        <div class="section-heading-top">
          <h3>ToolPkg 信息</h3>
          <span class="section-pill">manifest.json</span>
        </div>
        
      </div>`;
    const metaGrid = document.createElement("div");
    metaGrid.className = "metadata-grid";
    const themeIdField = this.input("theme-id", "org.example.theme");
    themeIdField.value = "org.operit.custom-theme";
    const versionField = this.input("version", "1.0.0");
    versionField.value = "1.0.0";
    const displayNameField = this.input("display-name", "My Theme");
    displayNameField.value = "Custom Studio Theme";
    const authorField = this.input("author", "Creator Name");
    authorField.value = "Operit Creator";
    metaGrid.append(
      this.labeled("ToolPkg ID（toolpkg_id）", themeIdField),
      this.labeled("语义化版本", versionField),
      this.labeled("ToolPkg 显示名称", displayNameField),
      this.labeled("创作者署名", authorField),
    );
    metadata.append(
      metaGrid,
      this.labeled("ToolPkg 简介（可选）", this.textarea("description", "简要介绍主题的视觉风格与适用场景", 2)),
    );
    const licTextArea = this.textarea("theme-license-text", "粘贴或编写 ToolPkg 的完整许可证文本", 5);
    metadata.append(this.labeled("ToolPkg 协议文本（manifest.license 引用 licenses/theme.txt）", licTextArea));

    form.append(metadata);

    const advanced = document.createElement("details");
    advanced.className = "advanced-json dock-section";
    advanced.dataset.section = "json";
    advanced.open = true;
    const summary = document.createElement("summary");
    summary.innerHTML = `
      <div class="summary-heading">
        <span class="summary-title">JSON</span>
        
      </div>
      <code class="summary-path">theme.json</code>`;
    this.themeJson.id = "theme-json";
    this.themeJson.rows = 22;
    this.themeJson.value = this.visualEditor.json;
    advanced.append(summary, this.labeled("theme.json 声明源码", this.themeJson));
    form.append(advanced);

    const renderer = document.createElement("details");
    renderer.className = "renderer-section";
    renderer.open = true;
    const rendererSummary = document.createElement("summary");
    rendererSummary.textContent = "本地 renderer 与契约 worker";
    this.rendererUrl.readOnly = true;
    this.workerUrl.readOnly = true;
    this.rendererRelease.readOnly = true;
    renderer.append(
      rendererSummary,
      this.labeled("Preview URL", this.rendererUrl),
      this.labeled("Contract worker URL", this.workerUrl),
      this.labeled("Renderer release", this.rendererRelease),
    );
    const rendererHost = this.root.querySelector<HTMLElement>(".renderer-fields-host");
    if (rendererHost != null) {
      rendererHost.append(renderer);
    } else {
      form.append(renderer);
    }

    const actions = document.createElement("div");
    actions.className = "actions header-actions";
    for (const btn of [
      this.undoButton,
      this.redoButton,
      this.resetDefaultButton,
      this.importButton,
      this.saveButton,
      this.loadButton,
      this.renderButton,
      this.exportButton,
    ]) {
      btn.type = "button";
    }

    this.undoButton.className = "ghost-btn icon-btn";
    this.undoButton.textContent = "↶";
    this.undoButton.title = "撤销上一步修改 (Ctrl+Z)";
    this.undoButton.disabled = true;

    this.redoButton.className = "ghost-btn icon-btn";
    this.redoButton.textContent = "↷";
    this.redoButton.title = "重做修改 (Ctrl+Y)";
    this.redoButton.disabled = true;

    this.resetDefaultButton.className = "ghost-btn";
    this.resetDefaultButton.textContent = "重置";
    this.resetDefaultButton.title = "将当前主题视觉参数恢复为官方默认主题";

    this.importButton.className = "secondary";
    this.importButton.textContent = "导入";
    this.importButton.title = "读取 ToolPkg 中的 theme 声明或主题 JSON 为草稿；导出生成新的 ToolPkg，不改写原包的其他能力";

    this.saveButton.className = "ghost-btn save-status-btn";
    this.saveButton.dataset.saveState = "saved";
    this.saveButton.textContent = "已保存";
    this.saveButton.title = "修改自动保存在本机；点击立即保存";
    this.loadButton.className = "ghost-btn";
    this.loadButton.textContent = "草稿";

    this.renderButton.className = "secondary";
    this.renderButton.textContent = "刷新";

    this.exportButton.className = "primary";
    this.exportButton.textContent = "导出 ToolPkg";
    this.exportButton.disabled = true;

    actions.append(
      this.undoButton,
      this.redoButton,
      this.resetDefaultButton,
      this.importButton,
      this.saveButton,
      this.loadButton,
      this.renderButton,
      this.exportButton,
    );
    this.root.querySelector(".header-actions-slot")!.replaceWith(actions);

    for (const id of METADATA_IDS) {
      const el = form.querySelector<HTMLInputElement | HTMLTextAreaElement>(`#${id}`);
      if (el) {
        el.addEventListener("input", () => {
          this.updateLiveThemeBadge();
          this.scheduleAtomicAutoSave();
        });
      }
    }

    this.renderAssetRows();
    this.applyDockTabVisibility();
  }

  private switchDockTab(tab: DockTabId): void {
    this.activeDockTab = tab;
    for (const btn of Array.from(this.dockTabBar.querySelectorAll<HTMLButtonElement>(".dock-tab-btn"))) {
      btn.classList.toggle("active", btn.dataset.dockTab === tab);
    }
    this.applyDockTabVisibility();
  }

  private applyDockTabVisibility(): void {
    const sections = Array.from(this.root.querySelectorAll<HTMLElement>("#editor-form .dock-section"));
    for (const sec of sections) {
      const kind = sec.dataset.section;
      const visible = this.activeDockTab === "all" || kind === this.activeDockTab;
      sec.classList.toggle("dock-hidden", !visible);
      if (visible && sec instanceof HTMLDetailsElement) {
        sec.open = true;
      }
    }
  }

  private bindEvidenceTabs(): void {
    const tabBtns = Array.from(this.root.querySelectorAll<HTMLButtonElement>("[data-evidence-tab]"));
    const panes = Array.from(this.root.querySelectorAll<HTMLElement>("[data-evidence-pane]"));
    for (const btn of tabBtns) {
      btn.onclick = () => {
        const target = (btn.dataset.evidenceTab as EvidenceTabId) ?? "receipt";
        this.activeEvidenceTab = target;
        for (const b of tabBtns) b.classList.toggle("active", b === btn);
        for (const p of panes) p.classList.toggle("active", p.dataset.evidencePane === target);
      };
    }
  }

  private renderLayerChips(): void {
    this.renderPreviewInspectOverlay();
    this.layerChipBar.replaceChildren();
    for (const item of LAYER_CHIPS) {
      const chip = document.createElement("button");
      chip.type = "button";
      const isSelected = this.selectedInspectTargetId === item.id;
      const isChatMessageTarget = item.id === "chat.userMessage" || item.id === "chat.aiMessage";
      chip.className = `layer-chip${isSelected ? " active" : ""}${isChatMessageTarget && !this.showConversation ? " muted-scene" : ""}`;
      const tag = document.createElement("span");
      tag.className = "layer-chip-key";
      tag.textContent = item.shortKey;
      const text = document.createElement("span");
      text.className = "layer-chip-label";
      text.textContent = item.label;
      chip.append(text);
      chip.onclick = () => this.selectInspectorLayer(item.id);
      this.layerChipBar.append(chip);
    }
  }



  private renderPreviewInspectOverlay(): void {
    this.previewInspectOverlay.replaceChildren();
    const active = this.inspectEnabled;
    this.previewInspectOverlay.classList.toggle("active", active);
    if (!active) return;

    const compact = this.viewport.width < 600;
    const zones: Array<{
      id: string;
      label: string;
      rect: { left: string; top: string; width: string; height: string };
      zIndex: number;
    }> = [];

    if (!compact) {
      const sbPct = ((280 / this.viewport.width) * 100).toFixed(2);
      const chatPct = (100 - Number(sbPct)).toFixed(2);
      zones.push({
        id: "navigation.sidebar",
        label: "🧭 导航侧栏 (点击配置明暗双套色)",
        rect: { left: "0%", top: "0%", width: `${sbPct}%`, height: "100%" },
        zIndex: 2,
      });
      zones.push({
        id: "chat.topBar",
        label: "📌 对话顶栏 (点击配置明暗双套色)",
        rect: { left: `${sbPct}%`, top: "0%", width: `${chatPct}%`, height: "8.5%" },
        zIndex: 2,
      });
      zones.push({
        id: "background",
        label: "🖼️ 画布背景 (点击配置背景与双套底色)",
        rect: { left: `${sbPct}%`, top: "8.5%", width: `${chatPct}%`, height: "75.5%" },
        zIndex: 1,
      });
      if (this.showConversation) {
        zones.push({
          id: "chat.userMessage",
          label: "💬 用户消息气泡 (点击配置气泡与明暗双套色)",
          rect: { left: "65%", top: "41%", width: "22%", height: "8%" },
          zIndex: 3,
        });
        zones.push({
          id: "chat.aiMessage",
          label: "🤖 AI 消息气泡 (点击配置气泡与明暗双套色)",
          rect: { left: "26%", top: "50%", width: "44%", height: "8%" },
          zIndex: 3,
        });
      }
      zones.push({
        id: "chat.composer",
        label: "⌨️ 消息输入框 (点击配置输入框与明暗双套色)",
        rect: { left: `${sbPct}%`, top: "84%", width: `${chatPct}%`, height: "16%" },
        zIndex: 2,
      });
    } else {
      zones.push({
        id: "chat.topBar",
        label: "📌 对话顶栏 (点击配置明暗双套色)",
        rect: { left: "0%", top: "0%", width: "100%", height: "8.5%" },
        zIndex: 2,
      });
      zones.push({
        id: "background",
        label: "🖼️ 画布背景 (点击配置背景与双套底色)",
        rect: { left: "0%", top: "8.5%", width: "100%", height: "75.5%" },
        zIndex: 1,
      });
      if (this.showConversation) {
        zones.push({
          id: "chat.userMessage",
          label: "💬 用户消息气泡 (点击配置气泡与明暗双套色)",
          rect: { left: "26%", top: "33%", width: "68%", height: "11%" },
          zIndex: 3,
        });
        zones.push({
          id: "chat.aiMessage",
          label: "🤖 AI 消息气泡 (点击配置气泡与明暗双套色)",
          rect: { left: "5%", top: "45%", width: "78%", height: "11%" },
          zIndex: 3,
        });
      }
      zones.push({
        id: "chat.composer",
        label: "⌨️ 消息输入框 (点击配置输入框与明暗双套色)",
        rect: { left: "0%", top: "84%", width: "100%", height: "16%" },
        zIndex: 2,
      });
    }

    for (const zone of zones) {
      const btn = document.createElement("button");
      btn.type = "button";
      const isSelected = this.selectedInspectTargetId === zone.id;
      btn.className = `preview-hotspot-zone${isSelected ? " selected" : ""}`;
      btn.dataset.targetId = zone.id;
      btn.setAttribute("aria-label", zone.label);
      btn.style.left = zone.rect.left;
      btn.style.top = zone.rect.top;
      btn.style.width = zone.rect.width;
      btn.style.height = zone.rect.height;
      btn.style.zIndex = String(zone.zIndex);

      const tag = document.createElement("span");
      tag.className = "preview-hotspot-tag";
      tag.textContent = zone.label;
      btn.append(tag);

      btn.onclick = event => {
        event.preventDefault();
        event.stopPropagation();
        this.selectInspectorLayer(zone.id);
      };
      this.previewInspectOverlay.append(btn);
    }
  }

  private renderStageColorHud(): void {
    this.hudToggleButton.classList.toggle("selected", this.stageHudOpen);
    const targetId =
      this.selectedInspectTargetId != null && Object.hasOwn(COMPONENT_COLOR_MAP, this.selectedInspectTargetId)
        ? this.selectedInspectTargetId
        : "navigation.sidebar";

    if (!this.stageHudOpen) {
      this.stageColorHud.classList.add("hidden");
      this.stageColorHud.replaceChildren();
      this.resizePreview();
      requestAnimationFrame(() => this.resizePreview());
      return;
    }

    this.stageColorHud.classList.remove("hidden");
    this.stageColorHud.replaceChildren();

    // Quick component switcher bar at the top of the Stage Color HUD
    const hudNav = document.createElement("div");
    hudNav.className = "stage-hud-nav";
    const hudTargets: Array<{ id: string; label: string }> = [
      { id: "navigation.sidebar", label: "🧭 导航侧栏" },
      { id: "chat.topBar", label: "📌 对话顶栏" },
      { id: "chat.userMessage", label: "💬 用户气泡" },
      { id: "chat.aiMessage", label: "🤖 AI 气泡" },
      { id: "chat.composer", label: "⌨️ 输入框" },
      { id: "background", label: "🖼️ 画布底色" },
    ];
    for (const item of hudTargets) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = `stage-hud-tab${targetId === item.id ? " active" : ""}`;
      btn.textContent = item.label;
      btn.onclick = () => {
        this.selectedInspectTargetId = item.id;
        this.selectInspectorLayer(item.id);
      };
      hudNav.append(btn);
    }

    const card = this.visualEditor.renderStageQuickColorBar(targetId, () => {
      this.stageHudOpen = false;
      this.renderStageColorHud();
    });

    const headerSlot = card.querySelector(".stage-quick-nav-slot");
    if (headerSlot) {
      headerSlot.replaceWith(hudNav);
    } else {
      card.prepend(hudNav);
    }
    this.stageColorHud.append(card);
    this.resizePreview();
    requestAnimationFrame(() => this.resizePreview());
  }

  private selectInspectorLayer(targetId: string | null): void {
    this.selectedInspectTargetId = targetId;
    this.visualEditor.setInspectorTarget(targetId == null ? null : THEME_INSPECTOR_TARGETS[targetId] ?? null);
    this.renderStageColorHud();

    // Automatically switch to conversation scene when inspecting User or AI bubbles!
    if ((targetId === "chat.userMessage" || targetId === "chat.aiMessage") && !this.showConversation) {
      this.conversationToggle.checked = true;
      this.toggleConversationScene();
      return;
    }

    this.renderLayerChips();
    if (
      this.inspectEnabled &&
      this.channel != null &&
      this.currentReceipt?.revision === this.currentRevision &&
      (targetId == null || CANVAS_INSPECT_TARGET_IDS.has(targetId))
    ) {
      const canHighlightInScene =
        targetId == null ||
        this.showConversation ||
        (targetId !== "chat.userMessage" && targetId !== "chat.aiMessage");
      if (canHighlightInScene) {
        try {
          this.channel.setInspectMode({
            revision: this.currentRevision,
            enabled: true,
            targetId,
          });
        } catch {
          // Ignore if scene does not currently mount the target.
        }
      }
    }
    if (targetId != null) {
      const targetTitle = THEME_INSPECTOR_TARGETS[targetId]?.title ?? targetId;
      this.setStatus(`已定位「${targetTitle}」声明参数。`, "ok");
    } else {
      this.setStatus("已返回全部主题参数视图。", "ok");
    }
  }

  private updateLiveThemeBadge(): void {
    const name = this.value("display-name") || "未命名主题";
    const id = this.value("theme-id") || "org.example.theme";
    const ver = this.value("version") || "0.1.0";
    this.liveThemeBadge.textContent = name;
    this.liveThemeBadge.title = `${id}@${ver}`;
  }

  private updateStageMetaStrip(): void {
    this.stageMetaStrip.className = "stage-meta-strip";
    const fixtureTag = this.rendererConfig?.fixtureVersion ?? "client-shell-v7";
    const scaleTag = this.originalPixels ? "1:1 像素" : "自适应";
    const shaShort = this.currentThemeSha256 ? `#${this.currentThemeSha256.slice(0, 8)}` : "等待回执";
    this.stageMetaStrip.innerHTML = `
      <span class="meta-pill">${fixtureTag}</span>
      <span class="meta-pill">${this.viewport.width}×${this.viewport.height}</span>
      <span class="meta-pill">${scaleTag}</span>
      <span class="meta-pill mono">${shaShort}</span>`;
  }

  private bindActions(): void {
    this.undoButton.onclick = () => this.undo();
    this.redoButton.onclick = () => this.redo();
    this.resetDefaultButton.onclick = () => this.resetToDefaultTheme();
    this.importButton.onclick = () => this.promptImportThemeFile();
    this.renderButton.onclick = () => void this.renderDraft();
    this.exportButton.onclick = () => void this.exportDraft();
    this.saveButton.onclick = () => {
      clearTimeout(this.autoSaveTimer);
      this.themeJson.value = this.visualEditor.json;
      void this.saveLocalDraft(false);
    };
    this.loadButton.onclick = () => void this.loadLocalDraft();

    this.addAssetButton.type = "button";
    this.addAssetButton.className = "add-asset-cta";
    this.addAssetButton.textContent = "添加素材";
    this.addAssetButton.onclick = () => this.addAsset();

    this.cleanUnusedButton.type = "button";
    this.cleanUnusedButton.className = "clean-unused-cta";
    this.cleanUnusedButton.textContent = "清理未使用";
    this.cleanUnusedButton.onclick = () => this.cleanUnusedAssets();

    this.themeJson.oninput = () => this.invalidate("高级 JSON 已修改；请重新校验预览。");
    this.themeJson.onblur = () => {
      try {
        this.visualEditor.setJson(this.themeJson.value, false);
        this.pushHistorySnapshot(this.themeJson.value);
        this.renderAssetRows();
      } catch (error) {
        this.setStatus(`theme.json 仍不是可解析对象：${String(error)}`, "error");
      }
    };

    const previewActions = this.root.querySelector<HTMLElement>(".preview-actions")!;
    this.lightButton.type = this.darkButton.type = "button";
    this.lightButton.textContent = "浅色";
    this.darkButton.textContent = "深色";
    this.lightButton.onclick = () => this.switchBrightness("light");
    this.darkButton.onclick = () => this.switchBrightness("dark");
    this.updateBrightnessButtons();

    for (const preset of PREVIEW_VIEWPORTS) {
      const option = document.createElement("option");
      option.value = preset.id;
      option.textContent = preset.label;
      this.viewportSelect.append(option);
    }
    this.viewportSelect.setAttribute("aria-label", "预览视口");
    this.viewportSelect.value = "desktop";
    this.viewportSelect.onchange = () => {
      const preset = PREVIEW_VIEWPORTS.find(item => item.id === this.viewportSelect.value);
      if (preset == null) return;
      this.viewport = { ...preset.viewport };
      this.resizePreview();
      this.updateStageMetaStrip();
      this.invalidate(`视口已切换为 ${preset.label}；正在重新渲染。`);
      void this.renderDraft();
    };

    this.zoomButton.type = "button";
    this.zoomButton.textContent = "1:1";
    this.zoomButton.onclick = () => {
      this.originalPixels = !this.originalPixels;
      this.zoomButton.classList.toggle("selected", this.originalPixels);
      this.resizePreview();
      this.updateStageMetaStrip();
    };

    this.inspectButton.type = "button";
    this.inspectButton.id = "inspect-button";
    this.inspectButton.textContent = "点选";

    this.hudToggleButton.type = "button";
    this.hudToggleButton.className = "hud-toggle-btn";
    this.hudToggleButton.textContent = "配色面板";
    this.hudToggleButton.title = "在右侧预览舞台打开当前选中组件的「☀️ 亮色 + 🌙 暗色」双套色彩声明面板";
    this.hudToggleButton.onclick = () => {
      this.stageHudOpen = !this.stageHudOpen;
      if (this.stageHudOpen && (this.selectedInspectTargetId == null || !Object.hasOwn(COMPONENT_COLOR_MAP, this.selectedInspectTargetId))) {
        this.selectedInspectTargetId = "navigation.sidebar";
        this.visualEditor.setInspectorTarget(THEME_INSPECTOR_TARGETS["navigation.sidebar"] ?? null);
        this.renderLayerChips();
      }
      this.renderStageColorHud();
    };
    this.inspectButton.disabled = true;
    this.inspectButton.setAttribute("role", "checkbox");
    this.inspectButton.setAttribute("aria-checked", "false");
    this.inspectButton.onclick = () => this.toggleInspectMode();

    const conversationToggleLabel = document.createElement("label");
    conversationToggleLabel.className = "preview-scene-toggle";
    this.conversationToggle.type = "checkbox";
    this.conversationToggle.id = "show-conversation";
    this.conversationToggle.checked = this.showConversation;
    this.conversationToggle.disabled = true;
    this.conversationToggle.setAttribute("aria-label", "显示正常对话");
    this.conversationToggle.onchange = () => this.toggleConversationScene();
    const conversationText = document.createElement("span");
    conversationText.textContent = "对话";
    conversationToggleLabel.append(this.conversationToggle, conversationText);

    this.captureButton.type = this.downloadPreviewButton.type = "button";
    this.captureButton.id = "capture-button";
    this.captureButton.textContent = "截图";
    this.captureButton.disabled = true;
    this.captureButton.onclick = () => void this.capturePreviewPng();

    this.downloadPreviewButton.id = "download-preview-button";
    this.downloadPreviewButton.textContent = "下载 PNG";
    this.downloadPreviewButton.disabled = true;
    this.downloadPreviewButton.onclick = () => {
      if (this.previewPng == null) return;
      download(
        this.previewPng,
        `${this.value("theme-id") || "theme"}-preview-${this.brightness}.png`,
        "image/png",
      );
    };

    previewActions.append(
      this.inspectButton,
      this.hudToggleButton,
      conversationToggleLabel,
      this.lightButton,
      this.darkButton,
      this.viewportSelect,
      this.zoomButton,
      this.captureButton,
      this.downloadPreviewButton,
    );
  }

  private promptImportThemeFile(): void {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".toolpkg,.json,application/zip,application/json";
    input.onchange = () => {
      const file = input.files?.[0];
      if (file) void this.importToolPkgTemplateFile(file);
    };
    input.click();
  }

  /** Requires an explicit selection when a ToolPkg declares multiple themes. */
  private selectToolPkgTheme(themes: readonly ToolPkgThemeDeclaration[]): Promise<string> {
    const dialog = document.createElement("dialog");
    const form = document.createElement("form");
    form.method = "dialog";
    const heading = document.createElement("h3");
    heading.textContent = "选择 ToolPkg 中要编辑的 theme 声明";
    const select = document.createElement("select");
    select.setAttribute("aria-label", "Theme 声明");
    for (const theme of themes) {
      const option = document.createElement("option");
      option.value = theme.id;
      option.textContent = `${theme.id} — ${theme.path}`;
      select.append(option);
    }
    const cancel = document.createElement("button");
    cancel.value = "cancel";
    cancel.textContent = "取消";
    const confirm = document.createElement("button");
    confirm.className = "primary";
    confirm.value = "confirm";
    confirm.textContent = "编辑所选声明";
    form.append(heading, select, cancel, confirm);
    dialog.append(form);
    return new Promise((resolve, reject) => {
      dialog.addEventListener("close", () => {
        const confirmed = dialog.returnValue === "confirm";
        const selectedId = select.value;
        dialog.remove();
        if (confirmed) resolve(selectedId);
        else reject(new Error("已取消选择 theme 声明"));
      }, { once: true });
      this.root.append(dialog);
      dialog.showModal();
    });
  }

  /** Imports a selected ToolPkg theme declaration or a visual JSON draft. */
  private async importToolPkgTemplateFile(file: File): Promise<void> {
    try {
      this.setStatus(`正在解析导入文件 ${file.name}…`, "warning");
      const bundle = await parseImportedThemeFile(file, async (themes) => this.selectToolPkgTheme(themes));
      this.importedToolPkg = bundle.source;

      if (bundle.metadata != null) {
        /** Clears absent optional metadata instead of retaining values from an earlier draft. */
        const setField = (id: string, val: string | undefined) => {
          const el = this.root.querySelector<HTMLInputElement | HTMLTextAreaElement>(`#${id}`);
          if (el == null) throw new Error(`Missing metadata field: ${id}`);
          el.value = "";
          if (val !== undefined) el.value = val;
        };
        setField("theme-id", bundle.metadata.themeId);
        setField("version", bundle.metadata.version);
        setField("display-name", bundle.metadata.displayName);
        setField("author", bundle.metadata.authorName);
        setField("description", bundle.metadata.description);
        setField("theme-license-text", bundle.metadata.themeLicenseText);
        this.updateLiveThemeBadge();
      }

      if (bundle.kind === "toolpkg") {
        this.rows.splice(0, this.rows.length);
        this.nextAssetId = 1;
        const referenced = referencedAssetPaths(bundle.themeJson);
        for (const item of bundle.assets.filter(asset => referenced.has(asset.path))) {
          const detected = sniffAssetFormat(item.bytes, item.path);
          const cleanBytes =
            detected != null ? sanitizeImageBytesLossless(item.bytes, detected.extension) : item.bytes;
          const sha = await digest(cleanBytes);
          const mediaType = detected?.mediaType ?? mediaTypeFor(item.path);
          this.rows.push({
            id: this.nextAssetId++,
            path: item.path,



            file: null,
            bytes: cleanBytes,
            sha256: sha,
            mediaType,
          });
        }
      }

      this.visualEditor.setJson(bundle.themeJson, false);
      this.themeJson.value = this.visualEditor.json;
      this.pushHistorySnapshot(this.themeJson.value);
      this.refreshEditorAssets();
      this.renderAssetRows();
      await this.saveLocalDraft();
      this.setStatus(
        bundle.kind === "toolpkg"
          ? `已读取 ToolPkg ${file.name} 的 theme 声明（含 ${this.rows.length} 个引用素材）；导出将生成新的 ToolPkg。正在渲染预览…`
          : `已导入主题声明 ${file.name}；正在渲染预览…`,
        "ok",
      );
      await this.renderDraft();
    } catch (error) {
      this.setStatus(`导入失败：${String(error)}`, "error");
    }
  }

  private cleanUnusedAssets(): void {
    const refMap = collectAssetReferenceMap(this.themeJson.value);
    const beforeCount = this.rows.length;
    for (let i = this.rows.length - 1; i >= 0; i -= 1) {
      const row = this.rows[i];
      if (!refMap.has(row.path)) {
        this.rows.splice(i, 1);
      }
    }
    const removed = beforeCount - this.rows.length;
    this.refreshEditorAssets();
    this.renderAssetRows();
    if (removed > 0) {
      void this.saveLocalDraft();
      this.setStatus(`已清理 ${removed} 个未被当前主题引用的冗余素材。`, "ok");
    } else {
      this.setStatus("当前所有素材均已被主题引用，无需清理。", "ok");
    }
  }

  private switchBrightness(value: "light" | "dark"): void {
    if (this.brightness === value) return;
    if (this.inspectEnabled && this.channel?.connected && this.currentReceipt?.revision === this.currentRevision) {
      try {
        this.channel.setInspectMode({ revision: this.currentRevision, enabled: false });
        this.inspectAfterRender = true;
      } catch {
        // Ignore
      }
      this.inspectEnabled = false;
    }
    this.themeJson.value = this.visualEditor.json;
    this.brightness = value;
    this.visualEditor.setPaletteMode(value);
    this.updateBrightnessButtons();
    this.updateStageMetaStrip();
    this.invalidate("预览模式已切换；正在重新校验和渲染当前草稿。");
    void this.renderDraft();
  }

  private updateBrightnessButtons(): void {
    this.lightButton.classList.toggle("selected", this.brightness === "light");
    this.darkButton.classList.toggle("selected", this.brightness === "dark");
  }

  private async initialize(): Promise<void> {
    try {
      const response = await fetch("./theme-studio-config.json", { cache: "no-store" });
      if (!response.ok) throw new Error(`config HTTP ${response.status}`);
      const config = (await response.json()) as Partial<RendererConfig>;
      if (
        typeof config.rendererReleaseId !== "string" ||
        typeof config.previewUrl !== "string" ||
        typeof config.workerUrl !== "string" ||
        typeof config.fixtureVersion !== "string" ||
        !isSha256(config.fixtureSourceSha256) ||
        typeof config.rendererCommit !== "string" ||
        typeof config.flutterRevision !== "string" ||
        typeof config.sourceDirty !== "boolean" ||
        !/^[a-f0-9]{40}$/.test(config.rendererCommit) ||
        !/^[a-f0-9]{40}$/.test(config.flutterRevision) ||
        config.ready !== true
      ) {
        throw new Error(config.error ?? "pinned Flutter renderer release is not available");
      }
      const previewUrl = new URL(config.previewUrl, window.location.href);
      const workerUrl = new URL(config.workerUrl, window.location.href);
      if (
        previewUrl.origin !== window.location.origin ||
        workerUrl.origin !== window.location.origin ||
        !/^[A-Za-z0-9._-]{1,128}$/.test(config.rendererReleaseId)
      ) {
        throw new Error("renderer and worker must be same-origin pinned release URLs");
      }
      this.rendererConfig = {
        ready: true,
        rendererReleaseId: config.rendererReleaseId,
        previewUrl: previewUrl.href,
        workerUrl: workerUrl.href,
        fixtureVersion: config.fixtureVersion,
        fixtureSourceSha256: config.fixtureSourceSha256,
        rendererCommit: config.rendererCommit,
        flutterRevision: config.flutterRevision,
        sourceDirty: config.sourceDirty,
      };
      this.rendererUrl.value = previewUrl.href;
      this.workerUrl.value = workerUrl.href;
      this.rendererRelease.value = config.rendererReleaseId;
      this.setConnectionState("renderer ready");
      this.updateStageMetaStrip();
      this.setStatus(
        config.sourceDirty
          ? "本地 renderer 已就绪（开发态源码）；正在加载草稿…"
          : "Renderer 已固定；正在加载草稿…",
        "ok",
      );
      await this.loadLocalDraft(true);
    } catch (error) {
      this.rendererConfig = null;
      this.setConnectionState("renderer unavailable");
      this.setStatus(String(error), "error");
    }
  }

  private async bridge(): Promise<StudioContractBridge> {
    await this.rendererConfigPromise;
    if (this.rendererConfig == null) throw new Error("固定 Flutter renderer/WASM release 尚未生成，不能校验或导出。");
    if (this.contract == null) {
      this.contract = new StudioContractBridge(this.rendererConfig.workerUrl, this.rendererConfig.rendererReleaseId);
    }
    return this.contract;
  }

  private async renderDraft(): Promise<void> {
    clearTimeout(this.previewDebounceTimer);
    this.previewDebounceTimer = undefined;
    if (typeof document !== "undefined" && document.visibilityState === "hidden") {
      // Defer iframe CanvasKit render while browser tab is hidden so rAF suspension cannot cause timeout
      this.previewQueued = true;
      return;
    }
    if (this.busy || this.iframeRendering) {
      this.previewQueued = true;
      return;
    }
    this.busy = true;
    this.renderButton.disabled = true;
    this.updateInspectControls();
    const generation = this.generation;
    try {
      this.setConnectionState("validating draft");
      const bridge = await this.bridge();
      if (generation !== this.generation) {
        this.previewQueued = true;
        return;
      }
      const source = new TextEncoder().encode(this.themeJson.value);
      const normalized = JSON.parse(await bridge.normalizeDraft(source)) as Record<string, unknown>;
      if (generation !== this.generation) {
        this.previewQueued = true;
        return;
      }
      if (
        normalized.ok !== true ||
        typeof normalized.canonicalThemeJson !== "string" ||
        !isSha256(normalized.themeSha256)
      ) {
        throw new Error(JSON.stringify(normalized.errors ?? "theme draft is invalid"));
      }
      const canonical = normalized.canonicalThemeJson;
      const assets = await this.assetInputs(canonical);
      if (generation !== this.generation) {
        this.previewQueued = true;
        return;
      }
      const previewCheck = JSON.parse(
        await bridge.validatePreview(new TextEncoder().encode(canonical), assets),
      ) as Record<string, unknown>;
      if (generation !== this.generation) {
        this.previewQueued = true;
        return;
      }
      if (previewCheck.ok !== true || previewCheck.themeSha256 !== normalized.themeSha256) {
        throw new Error(JSON.stringify(previewCheck.errors ?? "preview contract rejected assets"));
      }
      this.themeJson.value = canonical;
      if (this.activeDockTab === "json") {
        this.visualEditor.syncCanonicalJson(canonical);
      }
      this.currentThemeSha256 = normalized.themeSha256 as string;
      this.currentReceipt = null;
      this.previewPng = null;
      this.latestAssets = assets;
      this.downloadPreviewButton.disabled = true;
      this.updateStageMetaStrip();
      if (this.channel === null || this.channel.failed) {
        this.channel?.close();
        this.connectPreview();
      }
      if (this.channel === null) throw new Error("preview channel was not created");
      await this.channel.waitUntilReady();
      if (generation !== this.generation) throw new Error("草稿在预览启动期间改变；结果已弃用");
      this.currentRevision += 1;
      this.renderedGeneration = generation;
      this.iframeRendering = true;
      this.channel.loadTheme({
        revision: this.currentRevision,
        canonicalThemeJson: canonical,
        themeSha256: this.currentThemeSha256,
        assets,
        brightness: this.brightness,
        showConversation: this.showConversation,
        viewport: this.viewport,
      });
      this.inspectEnabled = false;
      this.updateInspectControls();
      this.setConnectionState(`rendering rev ${this.currentRevision}`);
      this.setStatus("Rust/WASM 校验通过；等待 Flutter 渲染回执…");
    } catch (error) {
      if (generation === this.generation) {
        this.currentReceipt = null;
        this.previewPng = null;
        this.exportButton.disabled = true;
        this.setConnectionState("validation failed");
        this.setStatus(`校验或预览失败：${String(error)}`, "error");
      }
    } finally {
      this.busy = false;
      this.renderButton.disabled = false;
      this.updateInspectControls();
      if (!this.iframeRendering && this.previewQueued && (typeof document === "undefined" || document.visibilityState !== "hidden")) {
        this.previewQueued = false;
        void this.renderDraft();
      }
    }
  }

  private connectPreview(): void {
    if (this.rendererConfig == null) return;
    const nonce = randomNonce();
    this.channel = new ThemeStudioMessagePort({
      iframe: this.preview,
      previewUrl: this.rendererConfig.previewUrl,
      nonce,
      rendererReleaseId: this.rendererConfig.rendererReleaseId,
      fixtureVersion: this.rendererConfig.fixtureVersion,
      fixtureSourceSha256: this.rendererConfig.fixtureSourceSha256,
    });
    this.channel.connect(message => this.onPreviewMessage(message));
  }

  private onPreviewMessage(message: ThemeStudioMessage): void {
    this.messages.textContent = JSON.stringify(
      message,
      (_, value: unknown) => (value instanceof Uint8Array ? `<${value.byteLength} binary bytes>` : value),
      2,
    );
    switch (message.type) {
      case "operit.theme.preview.ready.v1":
        this.currentRevision = Math.max(this.currentRevision, message.nextRevision - 1);
        this.setConnectionState(`ready / next ${message.nextRevision}`);
        break;
      case "operit.theme.preview.progress.v1":
        this.setConnectionState(`${message.stage} / rev ${message.revision}`);
        break;
      case "operit.theme.preview.loaded.v1":
        this.setConnectionState(`loaded rev ${message.revision}`);
        break;
      case "operit.theme.preview.rendered.v1":
        this.iframeRendering = false;
        if (this.previewQueued && (typeof document === "undefined" || document.visibilityState !== "hidden")) {
          this.previewQueued = false;
          void this.renderDraft();
          break;
        }
        if (
          message.revision !== this.currentRevision ||
          this.renderedGeneration !== this.generation ||
          this.currentThemeSha256 !== message.receipt.themeSha256 ||
          message.receipt.showConversation !== this.showConversation
        ) {
          break;
        }
        this.currentReceipt = message.receipt;
        this.previewPng = null;
        this.downloadPreviewButton.disabled = true;
        this.exportButton.disabled = false;
        this.setConnectionState(`rendered rev ${message.revision}`);
        this.receipt.textContent = JSON.stringify(message.receipt, null, 2);
        this.updateStageMetaStrip();
        this.setStatus(
          `预览已更新`,
          "ok",
        );
        if (this.inspectAfterRender || this.autoInspectMode) {
          this.inspectAfterRender = false;
          this.inspectEnabled = true;
        }
        if (this.inspectEnabled) {
          const canvasTarget =
            this.selectedInspectTargetId != null && CANVAS_INSPECT_TARGET_IDS.has(this.selectedInspectTargetId)
              ? this.selectedInspectTargetId
              : null;
          this.channel?.setInspectMode({
            revision: message.revision,
            enabled: true,
            targetId: canvasTarget,
          });
        }
        this.updateInspectControls();
        break;
      case "operit.theme.preview.selection.v1":
        if (!this.inspectEnabled || message.enabled !== true) break;
        if (message.targetId == null) {
          this.updateInspectControls();
          break;
        }
        this.selectedInspectTargetId = message.targetId;
        this.SwitchToInspectorOnSelection(message.targetId);
        this.renderStageColorHud();
        this.updateInspectControls();
        this.setStatus(
          `已点选画布组件「${THEME_INSPECTOR_TARGETS[message.targetId]?.title ?? message.targetId}」，可在左侧编辑。`,
          "ok",
        );
        break;
      case "operit.theme.preview.error.v1":
        this.iframeRendering = false;
        this.currentReceipt = null;
        this.previewPng = null;
        this.exportButton.disabled = true;
        this.inspectEnabled = false;
        this.downloadPreviewButton.disabled = true;
        this.updateInspectControls();
        this.setConnectionState(`error: ${message.code}`);
        this.setStatus(`Flutter preview error：${message.message}`, "error");
        break;
    }
  }

  private SwitchToInspectorOnSelection(targetId: string | null): void {
    if (this.activeDockTab !== "inspector" && this.activeDockTab !== "all") {
      this.switchDockTab("inspector");
    }
    this.visualEditor.setInspectorTarget(
      targetId == null ? null : THEME_INSPECTOR_TARGETS[targetId] ?? null,
    );
    this.renderLayerChips();
  }

  private toggleInspectMode(): void {
    const channel = this.channel;
    if (
      channel == null ||
      this.currentReceipt == null ||
      this.currentReceipt.revision !== this.currentRevision ||
      this.renderedGeneration !== this.generation
    ) {
      this.setStatus("先完成当前主题的 Flutter 预览，再开启画布元素点选。", "warning");
      return;
    }
    const enabled = !this.inspectEnabled;
    const canvasTarget =
      enabled && this.selectedInspectTargetId != null && CANVAS_INSPECT_TARGET_IDS.has(this.selectedInspectTargetId)
        ? this.selectedInspectTargetId
        : null;
    try {
      channel.setInspectMode({
        revision: this.currentRevision,
        enabled,
        targetId: canvasTarget,
      });
      this.inspectEnabled = enabled;
      this.autoInspectMode = enabled;
      this.updateInspectControls();
      if (!enabled) {
        this.inspectAfterRender = false;
        this.stageHudOpen = false;
        this.renderStageColorHud();
      }
      this.setStatus(
        enabled
          ? "画布点选模式已开启；点击右侧预览中的组件即可定位左侧参数。"
          : "画布点选模式已关闭。",
        "ok",
      );
    } catch (error) {
      this.setStatus(`Inspector 无法切换：${String(error)}`, "error");
    }
  }

  private updateInspectControls(): void {
    const rendered =
      this.currentReceipt != null &&
      this.currentReceipt.revision === this.currentRevision &&
      this.renderedGeneration === this.generation;
    this.inspectButton.disabled = !rendered || this.busy;
    this.inspectButton.classList.toggle("selected", this.inspectEnabled);
    this.inspectButton.setAttribute("aria-checked", String(this.inspectEnabled));
    this.inspectButton.textContent = "点选";
    this.captureButton.disabled = !rendered || this.busy;
    this.hudToggleButton.classList.toggle("selected", this.stageHudOpen);
    // Allow toggling conversation scene even if inspect mode is active (toggleConversationScene handles it seamlessly)
    this.conversationToggle.disabled = !rendered || this.busy;
    this.renderPreviewInspectOverlay();
  }

  private toggleConversationScene(): void {
    const next = this.conversationToggle.checked;
    if (
      this.currentReceipt == null ||
      this.channel == null ||
      this.busy ||
      this.currentReceipt.revision !== this.currentRevision ||
      this.renderedGeneration !== this.generation
    ) {
      this.showConversation = next;
      this.renderLayerChips();
      this.updateStageMetaStrip();
      void this.renderDraft();
      return;
    }
    try {
      if (this.inspectEnabled) {
        this.channel.setInspectMode({ revision: this.currentRevision, enabled: false });
        this.inspectEnabled = false;
        this.inspectAfterRender = true;
      }
      this.iframeRendering = true;
      this.channel.setConversationMode({ revision: this.currentRevision, showConversation: next });
      this.showConversation = next;
      this.currentReceipt = null;
      this.previewPng = null;
      this.downloadPreviewButton.disabled = true;
      this.exportButton.disabled = true;
      this.captureButton.textContent = "截图";
      this.setConnectionState("rendering scene");
      this.renderLayerChips();
      this.updateStageMetaStrip();
      this.setStatus(
        next
          ? "正在切换为正常对话场景（展示用户与 AI 气泡）…"
          : "正在切换为空白会话场景…",
        "warning",
      );
      this.updateInspectControls();
    } catch (error) {
      this.conversationToggle.checked = this.showConversation;
      this.setStatus(`预览场景切换失败：${String(error)}`, "error");
    }
  }

  private async capturePreviewPng(): Promise<void> {
    const wasInspecting = this.inspectEnabled;
    try {
      if (this.currentReceipt == null || this.channel == null || this.renderedGeneration !== this.generation) {
        throw new Error("先完成当前草稿的 Flutter 渲染");
      }
      if (wasInspecting) {
        this.channel.setInspectMode({ revision: this.currentRevision, enabled: false });
        this.inspectEnabled = false;
        await new Promise(resolve => setTimeout(resolve, 60));
      }
      const generation = this.generation;
      const bytes = await this.channel.capturePng();
      if (generation !== this.generation || this.currentReceipt == null) throw new Error("捕获期间草稿已修改");
      this.previewPng = bytes;
      this.downloadPreviewButton.disabled = false;
      const brightnessLabel = this.currentReceipt.brightness === "light" ? "亮色" : "暗色";
      this.captureButton.textContent = `已捕获 (${Math.round(bytes.byteLength / 1024)} KB)`;
      this.setStatus(
        `revision ${this.currentReceipt.revision} 的 Flutter ${brightnessLabel}实际帧已捕获（${bytes.byteLength.toLocaleString()} B）。`,
        "ok",
      );
    } catch (error) {
      this.previewPng = null;
      this.downloadPreviewButton.disabled = true;
      this.setStatus(`捕获失败：${String(error)}`, "error");
    } finally {
      if (wasInspecting && this.channel?.connected && this.currentReceipt?.revision === this.currentRevision) {
        const canvasTarget =
          this.selectedInspectTargetId != null && CANVAS_INSPECT_TARGET_IDS.has(this.selectedInspectTargetId)
            ? this.selectedInspectTargetId
            : null;
        try {
          this.channel.setInspectMode({
            revision: this.currentRevision,
            enabled: true,
            targetId: canvasTarget,
          });
          this.inspectEnabled = true;
          this.updateInspectControls();
        } catch {
          // Ignore
        }
      }
    }
  }

  private async waitForCurrentRender(timeoutMs = 15000): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      if (
        !this.busy &&
        this.currentReceipt != null &&
        this.currentReceipt.revision === this.currentRevision &&
        this.renderedGeneration === this.generation &&
        this.currentReceipt.showConversation === this.showConversation &&
        this.currentReceipt.brightness === this.brightness &&
        sameViewport(this.currentReceipt.viewport, this.viewport)
      ) {
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 120));
    }
    throw new Error("等待桌面 1280×800 亮色正常对话预览帧渲染超时");
  }

  private async exportDraft(): Promise<void> {
    try {
      this.exportButton.disabled = true;
      if (this.inspectEnabled) {
        this.toggleInspectMode();
      }
      const needsSceneSwitch =
        !this.showConversation ||
        this.brightness !== "light" ||
        !sameViewport(this.viewport, { width: 1280, height: 800 }) ||
        this.currentReceipt == null ||
        this.renderedGeneration !== this.generation;

      if (needsSceneSwitch) {
        this.setStatus("正在自动切换为桌面 1280×800 亮色正常对话场景以生成标准 preview.png…", "warning");
        this.showConversation = true;
        this.conversationToggle.checked = true;
        this.brightness = "light";
        this.updateBrightnessButtons();
        this.viewport = { width: 1280, height: 800 };
        this.viewportSelect.value = "desktop";
        this.resizePreview();
        this.updateStageMetaStrip();
        this.renderLayerChips();
        this.invalidate("正在渲染导出所需的 1280×800 亮色正常对话帧…");
        await this.renderDraft();
      }
      await this.waitForCurrentRender();

      if (this.previewPng == null) {
        this.setStatus("正在自动捕获当前 1280×800 亮色 Flutter 预览帧…", "warning");
        await this.capturePreviewPng();
      }

      const generation = this.generation;
      const receipt = this.currentReceipt;
      const preview = this.previewPng;
      if (receipt == null || preview == null || this.currentThemeSha256 == null) {
        throw new Error("未能获取当前 Flutter 预览帧或渲染回执");
      }

      const bridge = await this.bridge();
      const config = this.rendererConfig;
      if (config == null) throw new Error("renderer release 不可用");

      const rawThemeId = this.metadataValue("theme-id");
      validateToolPkgId(rawThemeId);
      const validThemeId = rawThemeId;
      const rawVersion = this.metadataValue("version");
      const validVersion = rawVersion;
      const validDisplayName = this.metadataValue("display-name");
      const validAuthor = this.metadataValue("author");
      const validThemeLicenseText = this.metadataValue("theme-license-text");
      const metadata = {
        toolpkgId: validThemeId,
        version: validVersion,
        displayName: validDisplayName,
        authorName: validAuthor,

        agreementText: validThemeLicenseText,
        description: this.metadataValue("description") || null,

      };

      const authored = await exportWithContract(bridge, {
        metadataJson: new TextEncoder().encode(JSON.stringify(metadata)),
        themeJson: new TextEncoder().encode(this.themeJson.value),
        previewPng: preview,
        assets: this.latestAssets,
      });
      const bytes = this.importedToolPkg == null ? authored : await updateToolPkgTheme(this.importedToolPkg, authored);
      if (this.importedToolPkg != null) {
        await bridge.validatePackage(bytes, this.importedToolPkg.manifestPath);
      }
      const rendered = this.channel?.receipt;
      if (
        generation !== this.generation ||
        rendered?.revision !== receipt.revision ||
        rendered.themeSha256 !== this.currentThemeSha256
      ) {
        throw new Error("导出期间当前 Flutter 帧或草稿发生变化");
      }
      download(bytes, `${metadata.toolpkgId}-${metadata.version}.toolpkg`);
      const versionInput = this.root.querySelector<HTMLInputElement>("#version");
      if (versionInput) {
        const match = /^(\d+\.\d+\.)(\d+)(.*)$/.exec(metadata.version);
        if (match) {
          versionInput.value = `${match[1]}${Number(match[2]) + 1}${match[3]}`;
          this.updateLiveThemeBadge();
        }
      }
      this.setStatus(
        `已成功导出 ${metadata.toolpkgId}-${metadata.version}.toolpkg（${bytes.byteLength.toLocaleString()} bytes）；已通过 模板资源校验，可直接导入客户端。`,
        "ok",
      );
    } catch (error) {
      this.setStatus(`导出被阻止：${String(error)}`, "error");
    } finally {
      this.exportButton.disabled = this.currentReceipt == null;
    }
  }

  private addAsset(): void {
    const id = this.nextAssetId++;
    const row: AssetRow = {
      id,
      path: `assets/images/asset-${id}.png`,



      file: null,
      bytes: null,
      sha256: null,
      mediaType: null,
    };
    this.rows.push(row);
    this.renderAssetRows();
    this.refreshEditorAssets();
    this.switchDockTab("assets");
    this.invalidate("素材行已添加；请选择文件并确认许可证。");
  }

  private requestAssetUpload(path: string[]): void {
    this.assetUploadTarget = [...path];
    const font = path[0] === "typography";
    const input = document.createElement("input");
    input.type = "file";
    input.accept = font ? ".ttf,font/ttf" : ".png,.jpg,.jpeg,.svg,image/png,image/jpeg,image/svg+xml";
    input.onchange = () => {
      const file = input.files?.[0];
      const target = this.assetUploadTarget;
      this.assetUploadTarget = null;
      if (file == null || target == null) return;
      const id = this.nextAssetId++;
      const defaultSpdx = font ? "OFL-1.1" : "CC0-1.0";
      const row: AssetRow = {
        id,
        path: "",



        file: null,
        bytes: null,
        sha256: null,
        mediaType: null,
      };
      this.rows.push(row);
      void this.readAsset(row, file, target, false).catch(error => {
        const idx = this.rows.indexOf(row);
        if (idx !== -1 && row.bytes == null) this.rows.splice(idx, 1);
        this.renderAssetRows();
        this.setStatus(`素材读取失败：${String(error)}`, "error");
      });
    };
    input.click();
  }

  private uniqueAssetPath(kind: "images" | "fonts", basename: string, extension: string, ignoreRow?: AssetRow): string {
    const base = `assets/${kind}/${basename}.${extension}`;
    if (!this.rows.some(row => row !== ignoreRow && row.path === base)) return base;
    for (let suffix = 2; suffix < 10000; suffix += 1) {
      const candidate = `assets/${kind}/${basename}-${suffix}.${extension}`;
      if (!this.rows.some(row => row !== ignoreRow && row.path === candidate)) return candidate;
    }
    throw new Error("同名素材过多，无法生成唯一包内路径");
  }

  private renderAssetRows(): void {
    this.assetList.replaceChildren();
    const refMap = collectAssetReferenceMap(this.themeJson.value);
    const unusedCount = this.rows.filter(r => !refMap.has(r.path)).length;
    this.assetCountBadge.textContent = String(this.rows.length);
    this.cleanUnusedButton.disabled = unusedCount === 0;
    this.cleanUnusedButton.textContent =
      unusedCount > 0 ? `🧹 清理未引用素材 (${unusedCount})` : "🧹 无冗余素材";

    if (this.rows.length === 0) {
      const empty = document.createElement("div");
      empty.className = "asset-empty-state";
      empty.innerHTML = `
        <div class="asset-empty-glyph">✦</div>
        <h4>尚未挂载静态素材或 TTF 字体</h4>
        <p>可在上方点击「+ 添加 PNG / JPEG / SVG / TTF」，或在视觉检视器中点击「上传素材」直接绑定路径。</p>`;
      this.assetList.append(empty);
      return;
    }

    for (const row of this.rows) {
      const refs = refMap.get(row.path) ?? [];
      const card = document.createElement("article");
      card.className = `asset-card${refs.length === 0 ? " asset-card-unused" : ""}`;

      const cardHeader = document.createElement("div");
      cardHeader.className = "asset-card-header";
      const previewWell = document.createElement("div");
      previewWell.className = "asset-preview-well";
      if (row.bytes != null && row.mediaType != null && row.mediaType.startsWith("image/")) {
        const img = document.createElement("img");
        img.className = "asset-thumb-img";
        const blobUrl = URL.createObjectURL(new Blob([Uint8Array.from(row.bytes).buffer], { type: row.mediaType }));
        img.src = blobUrl;
        img.alt = row.path;
        img.onload = () => URL.revokeObjectURL(blobUrl);
        previewWell.append(img);
      } else if (row.mediaType === "font/ttf" || row.path.endsWith(".ttf")) {
        previewWell.innerHTML = `<span class="asset-font-glyph">Aa</span>`;
      } else {
        previewWell.innerHTML = `<span class="asset-placeholder-glyph">IMG</span>`;
      }

      const headerInfo = document.createElement("div");
      headerInfo.className = "asset-header-info";

      const badgeRow = document.createElement("div");
      badgeRow.className = "asset-badge-row";
      const titleCode = document.createElement("code");
      titleCode.className = "asset-path-badge";
      titleCode.textContent = row.path || "未指定路径";
      const refBadge = document.createElement("span");
      refBadge.className = `asset-ref-badge${refs.length > 0 ? " active" : " unused"}`;
      refBadge.textContent = refs.length > 0 ? `已绑定: ${refs.join(" / ")}` : "未引用 (不打入导出包)";
      badgeRow.append(titleCode, refBadge);

      const fileInfo = document.createElement("small");
      fileInfo.className = "asset-file-meta";
      fileInfo.textContent =
        row.bytes == null
          ? "尚未选择文件"
          : `${row.bytes.byteLength.toLocaleString()} 字节`;
      headerInfo.append(badgeRow, fileInfo);

      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "danger-ghost-btn";
      remove.textContent = "移除";
      remove.onclick = () => {
        this.rows.splice(this.rows.indexOf(row), 1);
        this.renderAssetRows();
        this.refreshEditorAssets();
        this.invalidate("素材已移除；请重新校验。");
      };

      cardHeader.append(previewWell, headerInfo, remove);

      const path = this.input(`asset-path-${row.id}`, "assets/images/name.png");
      path.value = row.path;
      const file = document.createElement("input");
      file.type = "file";
      file.accept = ".png,.jpg,.jpeg,.svg,.ttf,image/png,image/jpeg,image/svg+xml,font/ttf";

      path.oninput = () => {
        const prev = row.path;
        row.path = path.value.trim();
        row.mediaType =
          (row.bytes != null ? sniffAssetFormat(row.bytes, row.path)?.mediaType : null) ?? mediaTypeFor(row.path);
        titleCode.textContent = row.path || "未指定路径";
        if (prev && row.path && prev !== row.path && this.themeJson.value.includes(`"${prev}"`)) {
          const updatedJson = this.themeJson.value.replaceAll(`"${prev}"`, `"${row.path}"`);
          this.themeJson.value = updatedJson;
          this.visualEditor.setJson(updatedJson, false);
        }
        this.refreshEditorAssets();
        this.invalidate("素材路径已修改；请重新校验。");
        this.schedulePreview();
      };
      file.onchange = () => void this.readAsset(row, file.files?.[0] ?? null);

      const grid = document.createElement("div");
      grid.className = "asset-fields-grid";
      grid.append(
        this.labeled("包内路径", path),
        this.labeled("替换文件", file),
      );
      card.append(cardHeader, grid);
      this.assetList.append(card);
    }
  }

  private async readAsset(
    row: AssetRow,
    file: File | null,
    target?: string[],
    stayInAssetsTab = true,
  ): Promise<void> {
    row.file = file;
    if (file == null) {
      row.bytes = null;
      row.sha256 = null;
      row.mediaType = null;
      this.renderAssetRows();
      this.invalidate();
      return;
    }
    const rawBytes = new Uint8Array(await file.arrayBuffer());
    const detected = sniffAssetFormat(rawBytes, file.name);
    if (detected == null) {
      row.bytes = null;
      row.sha256 = null;
      row.mediaType = null;
      this.renderAssetRows();
      this.setStatus(
        `文件「${file.name}」不是有效的 PNG / JPEG / 静态 SVG / TTF 字体。请勿上传 WebP、AVIF 或伪装后缀的文件。`,
        "error",
      );
      return;
    }

    const bytes = sanitizeImageBytesLossless(rawBytes, detected.extension);
    row.bytes = bytes;
    row.sha256 = await digest(bytes);

    const stem =
      file.name
        .replace(/\.[^.]+$/, "")
        .toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^[-_]+|[-_]+$/g, "") || "asset";

    if (
      !row.path ||
      /^assets\/(images|fonts)\/asset-\d+\.(png|jpg|jpeg|svg|ttf)$/.test(row.path) ||
      mediaTypeFor(row.path) !== detected.mediaType
    ) {
      row.path = this.uniqueAssetPath(detected.kind, stem, detected.extension, row);
    }
    row.mediaType = detected.mediaType;

    this.refreshEditorAssets();
    if (target !== undefined) {
      this.visualEditor.setPathValue(target, row.path, false);
      this.themeJson.value = this.visualEditor.json;
      this.pushHistorySnapshot(this.themeJson.value);
    }
    this.renderAssetRows();
    if (!stayInAssetsTab && this.activeDockTab === "assets") {
      this.switchDockTab("inspector");
    }
    const repairNotice = detected.repairedFromExtension
      ? `（检测到真实格式为 ${detected.extension.toUpperCase()}，已自动规范化路径后缀）`
      : "";
    this.invalidate(`${row.path} 已加载${repairNotice}；正在同步 Flutter 预览…`);
    await this.saveLocalDraft();
    await this.renderDraft();
  }


  private setSaveButtonState(state: "dirty" | "saving" | "saved"): void {
    this.saveButton.dataset.saveState = state;
    if (state === "dirty" || state === "saving") {
      this.saveButton.textContent = "保存中";
    } else {
      this.saveButton.textContent = "已保存";
    }
  }

  private scheduleAtomicAutoSave(): void {
    this.setSaveButtonState("dirty");
    clearTimeout(this.autoSaveTimer);
    this.autoSaveTimer = setTimeout(() => {
      this.autoSaveTimer = undefined;
      void this.saveLocalDraft(true);
    }, 180);
  }

  private async saveLocalDraft(silent = false): Promise<void> {
    try {
      await saveDraft({
        themeJson: this.themeJson.value,
        toolpkg: this.importedToolPkg == null ? undefined : {
          bytes: new Blob([Uint8Array.from(this.importedToolPkg.bytes).buffer]),
          templateId: this.importedToolPkg.template.id,
        },
        metadata: Object.fromEntries(METADATA_IDS.map(id => [id, this.metadataValue(id)])),
        assets: this.rows
          .filter(row => row.bytes != null && row.sha256 != null && row.mediaType != null)
          .map(row => ({
            path: row.path,



            mediaType: row.mediaType!,
            sha256: row.sha256!,
            bytes: new Blob([Uint8Array.from(row.bytes!).buffer]),
          })),
      } as { themeJson: string; metadata: Record<string, string>; assets: SavedAsset[] });
      this.setSaveButtonState("saved");
      if (!silent) {
        this.setStatus("所有区域的颜色与参数变更已原子写入当前浏览器 IndexedDB。", "ok");
      }
    } catch (error) {
      if (!silent) {
        this.setStatus(`本机草稿保存失败：${String(error)}`, "error");
      }
    }
  }

  private async loadLocalDraft(silent = false): Promise<void> {
    try {
      const draft = await loadDraft();
      if (draft == null) return;
      if (draft.toolpkg == null) this.importedToolPkg = undefined;
      else {
        const bundle = await parseImportedThemeFile(new File([draft.toolpkg.bytes], "saved.toolpkg"), async () => draft.toolpkg!.templateId);
        this.importedToolPkg = bundle.source;
      }
      this.visualEditor.setJson(draft.themeJson, false);
      this.themeJson.value = this.visualEditor.json;
      for (const id of METADATA_IDS) {
        const input = this.root.querySelector<HTMLInputElement | HTMLTextAreaElement>(`#${id}`);
        if (input) input.value = draft.metadata[id] ?? "";
      }
      this.updateLiveThemeBadge();
      this.rows.splice(0, this.rows.length);
      this.nextAssetId = 1;
      let currentJsonText = this.themeJson.value;
      let jsonRepaired = false;
      for (const saved of draft.assets) {
        const rawBytes = new Uint8Array(await saved.bytes.arrayBuffer());
        const detected = sniffAssetFormat(rawBytes, saved.path);
        const bytes = detected != null ? sanitizeImageBytesLossless(rawBytes, detected.extension) : rawBytes;
        const freshSha256 = await digest(bytes);
        let assetPath = saved.path;
        let mediaType = saved.mediaType;
        if (detected != null && (mediaTypeFor(assetPath) !== detected.mediaType || assetPath.includes("/_"))) {
          const cleanBase =
            assetPath
              .split("/")
              .at(-1)
              ?.replace(/\.[^.]+$/, "")
              .toLowerCase()
              .replace(/[^a-z0-9_-]+/g, "-")
              .replace(/^[-_]+|[-_]+$/g, "") || "asset";
          const nextPath = this.uniqueAssetPath(detected.kind, cleanBase, detected.extension);
          if (currentJsonText.includes(`"${assetPath}"`)) {
            currentJsonText = currentJsonText.replaceAll(`"${assetPath}"`, `"${nextPath}"`);
            jsonRepaired = true;
          }
          assetPath = nextPath;
          mediaType = detected.mediaType;
        }
        this.rows.push({
          id: this.nextAssetId++,
          ...saved,
          path: assetPath,
          mediaType,
          sha256: freshSha256,
          file: null,
          bytes,
        });
      }
      if (jsonRepaired) {
        this.visualEditor.setJson(currentJsonText, false);
        this.themeJson.value = this.visualEditor.json;
      }
      this.pushHistorySnapshot(this.themeJson.value);
      this.refreshEditorAssets();
      this.renderAssetRows();
      this.invalidate("已读取本机草稿；正在同步预览。");
      if (!silent) {
        this.setStatus("已读取本机草稿。", "ok");
        void this.renderDraft();
      }
    } catch (error) {
      if (!silent) this.setStatus(`本机草稿读取失败：${String(error)}`, "error");
    }
  }

  private getAssetPreviewUrl(path: string): string | null {
    return this.assetObjectUrls.get(path) ?? null;
  }

  private refreshEditorAssets(): void {
    for (const url of this.assetObjectUrls.values()) {
      URL.revokeObjectURL(url);
    }
    this.assetObjectUrls.clear();
    for (const row of this.rows) {
      if (row.path && row.bytes != null && row.mediaType != null && row.mediaType.startsWith("image/")) {
        const blobUrl = URL.createObjectURL(new Blob([Uint8Array.from(row.bytes).buffer], { type: row.mediaType }));
        this.assetObjectUrls.set(row.path, blobUrl);
      }
    }
    this.visualEditor.setAssets(this.rows.map(row => row.path).filter(path => path.length > 0));
  }

  private syncVisualJson(): void {
    this.themeJson.value = this.visualEditor.json;
    this.scheduleHistorySnapshot();
    this.scheduleAtomicAutoSave();
    this.renderAssetRows();
    this.invalidate("视觉参数已原子更新并保存；正在同步 Flutter 预览…");
    this.schedulePreview();
  }

  private schedulePreview(): void {
    clearTimeout(this.previewDebounceTimer);
    this.previewDebounceTimer = setTimeout(() => {
      this.previewDebounceTimer = undefined;
      if (this.rendererConfig == null) return;
      if (this.busy) {
        this.previewQueued = true;
        return;
      }
      void this.renderDraft();
    }, 350);
  }

  private invalidate(message = "草稿已修改；当前预览/导出凭证已更新。"): void {
    if (
      this.inspectEnabled &&
      this.channel?.connected &&
      this.currentReceipt?.revision === this.currentRevision
    ) {
      this.channel.setInspectMode({ revision: this.currentRevision, enabled: false });
      this.inspectAfterRender = true;
    }
    this.inspectEnabled = false;
    this.currentThemeSha256 = null;
    this.currentReceipt = null;
    this.previewPng = null;
    this.latestAssets = [];
    this.downloadPreviewButton.disabled = true;
    this.exportButton.disabled = true;
    this.generation += 1;
    this.updateInspectControls();
    this.updateStageMetaStrip();
    this.captureButton.textContent = "截图";
    this.setConnectionState("draft changed");
    this.setStatus(message, "warning");
  }

  private async assetInputs(canonicalJson = this.themeJson.value): Promise<ThemePreviewAsset[]> {
    const referenced = referencedAssetPaths(canonicalJson);
    const seen = new Set<string>();
    return this.rows
      .filter(row => {
        if (row.bytes == null || row.sha256 == null || row.mediaType == null) return false;
        if (!referenced.has(row.path) || seen.has(row.path)) return false;
        seen.add(row.path);
        return true;
      })
      .map(row => ({
        path: row.path,
        mediaType: row.mediaType!,
        sha256: row.sha256!,
        bytes: Uint8Array.from(row.bytes!),
      }));
  }

  /** Reads package metadata exactly as displayed, preserving agreement whitespace and original identity. */
  private metadataValue(id: typeof METADATA_IDS[number]): string {
    const field = this.root.querySelector<HTMLInputElement | HTMLTextAreaElement>(`#${id}`);
    if (field == null) throw new Error(`Missing package metadata field: ${id}`);
    return field.value;
  }

  private value(id: string): string {
    return this.root.querySelector<HTMLInputElement | HTMLTextAreaElement>(`#${id}`)?.value.trim() ?? "";
  }

  /** Show concise progress and retain full error details for inspection. */
  private setStatus(value: string, tone: "ok" | "error" | "warning" | "" = ""): void {
    this.status.textContent = tone === "error" || tone === "warning" ? value : tone === "ok" ? "预览已更新" : "更新中…";
    this.status.title = value;
    this.status.dataset.tone = tone;
    const ribbon = this.root.querySelector<HTMLElement>(".status-ribbon");
    if (ribbon) ribbon.dataset.tone = tone;
  }

  private setConnectionState(value: string): void {
    this.connectionState.textContent = value;
    this.connectionState.dataset.state = value.startsWith("rendered")
      ? "rendered"
      : value.startsWith("error") || value.includes("failed") || value.includes("unavailable")
        ? "error"
        : value.includes("validating") || value.includes("rendering")
          ? "busy"
          : "ready";
  }

  /** Fit the preview within the actual padded stage area. */
  private resizePreview(): void {
    const shell = this.root.querySelector<HTMLElement>(".stage-frame-shell");
    const viewport = this.root.querySelector<HTMLElement>(".preview-viewport");
    if (shell == null || viewport == null) return;
    const style = getComputedStyle(shell);
    const availW = Math.max(80, shell.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight));
    const availH = Math.max(80, shell.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom));
    const fitScale = Math.min(1, availW / this.viewport.width, availH / this.viewport.height);
    const scale = this.originalPixels ? 1 : Math.max(0.15, fitScale);
    const scaledW = Math.floor(this.viewport.width * scale);
    const scaledH = Math.floor(this.viewport.height * scale);

    this.preview.style.width = `${this.viewport.width}px`;
    this.preview.style.height = `${this.viewport.height}px`;
    this.preview.style.transform = `scale(${scale})`;
    this.previewCanvas.style.width = `${scaledW}px`;
    this.previewCanvas.style.height = `${scaledH}px`;
    viewport.classList.toggle("original-pixels", this.originalPixels);
    viewport.style.width = this.originalPixels ? "100%" : `${scaledW}px`;
    viewport.style.height = this.originalPixels ? "100%" : `${scaledH}px`;
  }

  private input(id: string, placeholder: string): HTMLInputElement {
    const input = document.createElement("input");
    input.id = id;
    input.placeholder = placeholder;
    return input;
  }

  private textarea(id: string, placeholder: string, rows: number): HTMLTextAreaElement {
    const input = document.createElement("textarea");
    input.id = id;
    input.placeholder = placeholder;
    input.rows = rows;
    return input;
  }

  private labeled(title: string, control: HTMLElement): HTMLElement {
    const wrapper = document.createElement("label");
    wrapper.className = "control";
    const caption = document.createElement("span");
    caption.textContent = title;
    wrapper.append(caption, control);
    return wrapper;
  }
}

function collectAssetReferenceMap(themeJsonSource: string): Map<string, string[]> {
  const map = new Map<string, string[]>();
  const addRef = (val: unknown, label: string) => {
    if (typeof val === "string" && val.startsWith("assets/")) {
      const list = map.get(val) ?? [];
      list.push(label);
      map.set(val, list);
    }
  };
  try {
    const parsed = JSON.parse(themeJsonSource) as Record<string, unknown>;
    const background = parsed.background as Record<string, unknown> | undefined;
    const chat = parsed.chat as Record<string, unknown> | undefined;
    const user = chat?.user as Record<string, unknown> | undefined;
    const userImg = user?.image as Record<string, unknown> | undefined;
    const ai = chat?.ai as Record<string, unknown> | undefined;
    const aiImg = ai?.image as Record<string, unknown> | undefined;
    const input = parsed.input as Record<string, unknown> | undefined;
    const inputImg = input?.image as Record<string, unknown> | undefined;
    const typography = parsed.typography as Record<string, unknown> | undefined;

    addRef(background?.asset, "画布背景");
    addRef(userImg?.asset, "用户气泡贴图");
    addRef(aiImg?.asset, "AI气泡贴图");
    addRef(inputImg?.asset, "输入框贴图");
    addRef(input?.settingsAsset, "设置图标");
    addRef(input?.attachmentAsset, "附件图标");
    addRef(input?.micAsset, "语音图标");
    addRef(input?.sendAsset, "发送图标");
    addRef(input?.stopAsset, "停止图标");
    addRef(typography?.global, "全局字体");
    addRef(typography?.user, "用户消息字体");
    addRef(typography?.ai, "AI消息字体");
  } catch {
    // Ignore parse errors while typing
  }
  return map;
}

function referencedAssetPaths(themeJsonSource: string): Set<string> {
  return new Set(collectAssetReferenceMap(themeJsonSource).keys());
}

/**
 * Losslessly strips extraneous trailing bytes after PNG IEND or JPEG EOI,
 * and strips non-essential JPEG APP1..APP15 / COM metadata segments.
 */
function sanitizeImageBytesLossless(
  bytes: Uint8Array,
  extension: "png" | "jpg" | "svg" | "ttf",
): Uint8Array {
  if (extension === "png") {
    let offset = 8;
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    while (offset + 12 <= bytes.byteLength) {
      const len = view.getUint32(offset, false);
      const type = String.fromCharCode(
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
      );
      const chunkEnd = offset + 12 + len;
      if (chunkEnd > bytes.byteLength) break;
      offset = chunkEnd;
      if (type === "IEND") {
        return offset < bytes.byteLength ? bytes.slice(0, offset) : bytes;
      }
    }
    return bytes;
  }

  if (extension === "jpg") {
    if (bytes.byteLength < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8) return bytes;
    const kept: Uint8Array[] = [bytes.subarray(0, 2)];
    let offset = 2;
    let modified = false;

    while (offset < bytes.byteLength) {
      if (bytes[offset] !== 0xff) break;
      while (offset < bytes.byteLength && bytes[offset] === 0xff) offset += 1;
      if (offset >= bytes.byteLength) break;
      const marker = bytes[offset];
      offset += 1;

      if (marker === 0xd9) {
        kept.push(Uint8Array.from([0xff, 0xd9]));
        if (offset < bytes.byteLength) modified = true;
        break;
      }

      if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd8)) {
        kept.push(Uint8Array.from([0xff, marker]));
        continue;
      }

      if (offset + 2 > bytes.byteLength) break;
      const segLen = (bytes[offset] << 8) | bytes[offset + 1];
      if (segLen < 2 || offset + segLen > bytes.byteLength) break;

      const isAppOrComment = (marker >= 0xe1 && marker <= 0xef) || marker === 0xfe;
      if (isAppOrComment) {
        modified = true;
      } else {
        kept.push(bytes.subarray(offset - 2, offset + segLen));
      }
      offset += segLen;

      if (marker === 0xda) {
        const scanStart = offset;
        while (offset < bytes.byteLength) {
          if (bytes[offset] !== 0xff) {
            offset += 1;
            continue;
          }
          const mStart = offset;
          while (offset < bytes.byteLength && bytes[offset] === 0xff) offset += 1;
          if (offset >= bytes.byteLength) return bytes;
          const m = bytes[offset];
          if (m === 0x00 || (m >= 0xd0 && m <= 0xd7)) {
            offset += 1;
            continue;
          }
          kept.push(bytes.subarray(scanStart, mStart));
          offset = mStart;
          break;
        }
      }
    }

    if (!modified) return bytes;
    const total = kept.reduce((acc, part) => acc + part.byteLength, 0);
    const out = new Uint8Array(total);
    let cursor = 0;
    for (const part of kept) {
      out.set(part, cursor);
      cursor += part.byteLength;
    }
    return out;
  }

  return bytes;
}

function sniffAssetFormat(
  bytes: Uint8Array,
  fileName: string,
): {
  kind: "images" | "fonts";
  extension: "png" | "jpg" | "svg" | "ttf";
  mediaType: ThemePreviewAsset["mediaType"];
  repairedFromExtension: string | null;
} | null {
  const nameExt = fileName.split(".").at(-1)?.toLowerCase() ?? "";
  if (
    bytes.byteLength >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47 &&
    bytes[4] === 0x0d &&
    bytes[5] === 0x0a &&
    bytes[6] === 0x1a &&
    bytes[7] === 0x0a
  ) {
    return {
      kind: "images",
      extension: "png",
      mediaType: "image/png",
      repairedFromExtension: nameExt !== "png" ? nameExt : null,
    };
  }
  if (bytes.byteLength >= 4 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return {
      kind: "images",
      extension: "jpg",
      mediaType: "image/jpeg",
      repairedFromExtension: nameExt !== "jpg" && nameExt !== "jpeg" ? nameExt : null,
    };
  }
  if (
    bytes.byteLength >= 12 &&
    bytes[0] === 0x00 &&
    bytes[1] === 0x01 &&
    bytes[2] === 0x00 &&
    bytes[3] === 0x00
  ) {
    return {
      kind: "fonts",
      extension: "ttf",
      mediaType: "font/ttf",
      repairedFromExtension: nameExt !== "ttf" ? nameExt : null,
    };
  }
  const headText = new TextDecoder("utf-8", { fatal: false })
    .decode(bytes.subarray(0, Math.min(bytes.byteLength, 512)))
    .trimStart();
  if (headText.startsWith("<svg") || (headText.startsWith("<?xml") && headText.includes("<svg"))) {
    return {
      kind: "images",
      extension: "svg",
      mediaType: "image/svg+xml",
      repairedFromExtension: nameExt !== "svg" ? nameExt : null,
    };
  }
  return null;
}

function mediaTypeFor(path: string): ThemePreviewAsset["mediaType"] | null {
  const lower = path.toLowerCase();
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".svg")) return "image/svg+xml";
  if (lower.endsWith(".ttf")) return "font/ttf";
  return null;
}

function isSha256(value: unknown): value is string {
  return typeof value === "string" && SHA256.test(value);
}

function sameViewport(left: PreviewViewport, right: PreviewViewport): boolean {
  return left.width === right.width && left.height === right.height;
}

async function digest(bytes: Uint8Array): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", Uint8Array.from(bytes).buffer as ArrayBuffer);
  return Array.from(new Uint8Array(hash), byte => byte.toString(16).padStart(2, "0")).join("");
}

function randomNonce(): string {
  const bytes = new Uint8Array(24);
  crypto.getRandomValues(bytes);
  return btoa(String.fromCharCode(...bytes)).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function download(bytes: Uint8Array, filename: string, mediaType = "application/zip"): void {
  const url = URL.createObjectURL(new Blob([Uint8Array.from(bytes).buffer], { type: mediaType }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

const app = document.querySelector<HTMLElement>("#app");
if (app == null) throw new Error("Studio root is missing");
new ThemeStudioApp(app);
