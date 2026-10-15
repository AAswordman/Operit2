export interface ColorPopoverSwatch {
  label: string;
  hex8: string;
  isTransparent?: boolean;
}

export const MONOCHROME_SWATCHES: ReadonlyArray<ColorPopoverSwatch> = [
  { label: "纯白 #FFFFFF", hex8: "#FFFFFFFF" },
  { label: "柔白 #F8FAFC", hex8: "#F8FAFCFF" },
  { label: "浅灰 #E2E8F0", hex8: "#E2E8F0FF" },
  { label: "银灰 #CBD5E1", hex8: "#CBD5E1FF" },
  { label: "中灰 #64748B", hex8: "#64748BFF" },
  { label: "深灰 #334155", hex8: "#334155FF" },
  { label: "炭黑 #1E293B", hex8: "#1E293BFF" },
  { label: "暗夜 #111318", hex8: "#111318FF" },
  { label: "深邃 #090D16", hex8: "#090D16FF" },
  { label: "纯黑 #000000", hex8: "#000000FF" },
  { label: "半透白 65%", hex8: "#FFFFFFA6" },
  { label: "半透黑 50%", hex8: "#00000080" },
  { label: "全透明 0%", hex8: "#00000000", isTransparent: true },
];

export const ACCENT_SWATCHES: ReadonlyArray<ColorPopoverSwatch> = [
  { label: "经典蓝 #36618E", hex8: "#36618EFF" },
  { label: "冰川蓝 #D1E4FF", hex8: "#D1E4FFFF" },
  { label: "亮天蓝 #38BDF8", hex8: "#38BDF8FF" },
  { label: "深海蓝 #0C4A6E", hex8: "#0C4A6EFF" },
  { label: "翡翠绿 #10B981", hex8: "#10B981FF" },
  { label: "薄荷浅绿 #D1FAE5", hex8: "#D1FAE5FF" },
  { label: "深林绿 #064E3B", hex8: "#064E3BFF" },
  { label: "琥珀金 #F59E0B", hex8: "#F59E0BFF" },
  { label: "羊皮纸暖白 #FAF6EE", hex8: "#FAF6EEFF" },
  { label: "深褐木色 #2E2012", hex8: "#2E2012FF" },
  { label: "紫晶 #8B5CF6", hex8: "#8B5CF6FF" },
  { label: "薰衣草浅紫 #F3E8FF", hex8: "#F3E8FFFF" },
  { label: "黑曜石深紫 #261933", hex8: "#261933FF" },
  { label: "珊瑚红 #F43F5E", hex8: "#F43F5EFF" },
];

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

export function rgbaCssFromHex8(hex8: string): string {
  if (!/^#[0-9A-Fa-f]{8}$/.test(hex8)) return "#808080";
  const r = parseInt(hex8.slice(1, 3), 16);
  const g = parseInt(hex8.slice(3, 5), 16);
  const b = parseInt(hex8.slice(5, 7), 16);
  const a = parseInt(hex8.slice(7, 9), 16) / 255;
  return `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
}

function hex8ToHsva(hex8: string): { h: number; s: number; v: number; a: number } {
  const clean = /^#[0-9A-Fa-f]{8}$/.test(hex8) ? hex8.toUpperCase() : "#3B82F6FF";
  const r = parseInt(clean.slice(1, 3), 16) / 255;
  const g = parseInt(clean.slice(3, 5), 16) / 255;
  const b = parseInt(clean.slice(5, 7), 16) / 255;
  const a = parseInt(clean.slice(7, 9), 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  const s = max === 0 ? 0 : d / max;
  const v = max;

  if (max !== min) {
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
        break;
    }
    h *= 60;
  }
  return { h, s: s * 100, v: v * 100, a: Math.round(a * 100) };
}

function hsvaToHex8(h: number, s: number, v: number, aPct: number): string {
  const hn = ((h % 360) + 360) % 360;
  const sn = clamp(s, 0, 100) / 100;
  const vn = clamp(v, 0, 100) / 100;

  const c = vn * sn;
  const x = c * (1 - Math.abs(((hn / 60) % 2) - 1));
  const m = vn - c;
  let r1 = 0;
  let g1 = 0;
  let b1 = 0;
  if (hn < 60) [r1, g1, b1] = [c, x, 0];
  else if (hn < 120) [r1, g1, b1] = [x, c, 0];
  else if (hn < 180) [r1, g1, b1] = [0, c, x];
  else if (hn < 240) [r1, g1, b1] = [0, x, c];
  else if (hn < 300) [r1, g1, b1] = [x, 0, c];
  else [r1, g1, b1] = [c, 0, x];

  const toHex = (n: number) =>
    clamp(Math.round((n + m) * 255), 0, 255)
      .toString(16)
      .toUpperCase()
      .padStart(2, "0");
  const aByte = clamp(Math.round((clamp(aPct, 0, 100) / 100) * 255), 0, 255)
    .toString(16)
    .toUpperCase()
    .padStart(2, "0");
  return `#${toHex(r1)}${toHex(g1)}${toHex(b1)}${aByte}`;
}

export interface ColorPopoverOpenOptions {
  anchor: HTMLElement;
  title: string;
  subtitle?: string;
  initialMode?: "light" | "dark";
  getHex: (mode: "light" | "dark") => string;
  onChange: (mode: "light" | "dark", nextHex8: string) => void;
  onReset?: (mode: "light" | "dark") => string | undefined;
  dualMode?: boolean;
}

class StudioColorPopover {
  private readonly el = document.createElement("div");
  private activeAnchor: HTMLElement | null = null;
  private options: ColorPopoverOpenOptions | null = null;
  private currentMode: "light" | "dark" = "light";
  private hsv = { h: 210, s: 70, v: 90, a: 100 };

  constructor() {
    this.el.className = "studio-color-popover hidden";
    document.body.append(this.el);

    window.addEventListener("pointerdown", event => {
      if (this.el.classList.contains("hidden")) return;
      const target = event.target as Node | null;
      if (target && (this.el.contains(target) || (this.activeAnchor && this.activeAnchor.contains(target)))) {
        return;
      }
      this.close();
    });

    window.addEventListener("keydown", event => {
      if (event.key === "Escape" && !this.el.classList.contains("hidden")) {
        this.close();
      }
    });

    window.addEventListener("resize", () => {
      if (!this.el.classList.contains("hidden") && this.activeAnchor) {
        this.positionNear(this.activeAnchor);
      }
    });
  }

  open(options: ColorPopoverOpenOptions): void {
    this.options = options;
    this.activeAnchor = options.anchor;
    this.currentMode = options.initialMode ?? "light";
    const initialHex = options.getHex(this.currentMode);
    this.hsv = hex8ToHsva(initialHex);
    this.render();
    this.el.classList.remove("hidden");
    this.positionNear(options.anchor);
  }

  close(): void {
    if (!this.el.classList.contains("hidden") && this.options) {
      const hexInput = this.el.querySelector<HTMLInputElement>(".cpop-hex-input");
      if (hexInput) {
        let clean = hexInput.value.trim().toUpperCase();
        if (!clean.startsWith("#")) clean = "#" + clean;
        if (/^#[0-9A-F]{6}$/.test(clean)) clean += "FF";
        if (/^#[0-9A-F]{8}$/.test(clean) && clean !== this.options.getHex(this.currentMode)) {
          this.options.onChange(this.currentMode, clean);
        }
      }
    }
    this.el.classList.add("hidden");
    this.activeAnchor = null;
    this.options = null;
  }

  private positionNear(anchor: HTMLElement): void {
    const rect = anchor.getBoundingClientRect();
    const popW = 296;
    const popH = this.el.offsetHeight || 395;
    let left = rect.left;
    let top = rect.bottom + 6;

    if (left + popW > window.innerWidth - 12) {
      left = Math.max(12, window.innerWidth - popW - 12);
    }
    if (top + popH > window.innerHeight - 12) {
      top = Math.max(12, rect.top - popH - 6);
    }
    this.el.style.left = `${Math.round(left)}px`;
    this.el.style.top = `${Math.round(top)}px`;
  }

  private render(): void {
    const opts = this.options;
    if (!opts) return;
    this.el.replaceChildren();

    const currentHex = opts.getHex(this.currentMode);
    this.hsv = hex8ToHsva(currentHex);

    // 1. Header Bar
    const header = document.createElement("div");
    header.className = "cpop-header";

    const titleWrap = document.createElement("div");
    titleWrap.className = "cpop-title-wrap";
    const titleEl = document.createElement("span");
    titleEl.className = "cpop-title";
    titleEl.textContent = opts.title;
    titleWrap.append(titleEl);
    if (opts.subtitle) {
      const subEl = document.createElement("code");
      subEl.className = "cpop-sub";
      subEl.textContent = opts.subtitle;
      titleWrap.append(subEl);
    }

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "cpop-close-btn";
    closeBtn.textContent = "✕";
    closeBtn.onclick = () => this.close();
    header.append(titleWrap, closeBtn);
    this.el.append(header);

    // 2. Optional Light / Dark Mode Switcher inside Popover
    if (opts.dualMode) {
      const modeSwitch = document.createElement("div");
      modeSwitch.className = "cpop-mode-switch";
      for (const m of ["light", "dark"] as const) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = `cpop-mode-btn${this.currentMode === m ? " active" : ""}`;
        const hexForM = opts.getHex(m);
        const dot = document.createElement("span");
        dot.className = "cpop-mode-dot";
        dot.style.background = rgbaCssFromHex8(hexForM);
        const txt = document.createElement("span");
        txt.textContent = m === "light" ? "浅色" : "深色";
        btn.append(dot, txt);
        btn.onclick = () => {
          this.currentMode = m;
          this.render();
        };
        modeSwitch.append(btn);
      }
      this.el.append(modeSwitch);
    }

    // 3. 2D Saturation / Value Spectrum Box
    const svPlane = document.createElement("div");
    svPlane.className = "cpop-sv-plane";
    const svThumb = document.createElement("div");
    svThumb.className = "cpop-sv-thumb";
    svPlane.append(svThumb);

    // 4. Hue & Alpha Sliders
    const slidersWrap = document.createElement("div");
    slidersWrap.className = "cpop-sliders";

    const hueInput = document.createElement("input");
    hueInput.type = "range";
    hueInput.className = "cpop-hue-slider";
    hueInput.min = "0";
    hueInput.max = "360";
    hueInput.step = "1";
    hueInput.value = String(Math.round(this.hsv.h));

    const alphaRow = document.createElement("div");
    alphaRow.className = "cpop-alpha-row";
    const alphaInput = document.createElement("input");
    alphaInput.type = "range";
    alphaInput.className = "cpop-alpha-slider";
    alphaInput.min = "0";
    alphaInput.max = "100";
    alphaInput.step = "1";
    alphaInput.value = String(this.hsv.a);
    const alphaValue = document.createElement("span");
    alphaValue.className = "cpop-alpha-val";
    alphaValue.textContent = `${this.hsv.a}%`;
    alphaRow.append(alphaInput, alphaValue);

    slidersWrap.append(hueInput, alphaRow);

    // 5. Hex Input + Native Picker + Reset Row
    const hexRow = document.createElement("div");
    hexRow.className = "cpop-hex-row";

    const previewChip = document.createElement("label");
    previewChip.className = "cpop-preview-chip";
    previewChip.title = "点击调用系统吸管取色器";
    const nativeColor = document.createElement("input");
    nativeColor.type = "color";
    nativeColor.className = "native-color-picker";
    nativeColor.value = currentHex.slice(0, 7);
    previewChip.append(nativeColor);

    const hexInput = document.createElement("input");
    hexInput.type = "text";
    hexInput.className = "cpop-hex-input";
    hexInput.value = currentHex;
    hexInput.maxLength = 9;

    const resetBtn = document.createElement("button");
    resetBtn.type = "button";
    resetBtn.className = "cpop-reset-btn";
    resetBtn.textContent = "↺ 默认";
    resetBtn.onclick = () => {
      const defHex = opts.onReset?.(this.currentMode);
      if (defHex) {
        this.hsv = hex8ToHsva(defHex);
        applyHsvChange(defHex);
      }
    };

    hexRow.append(previewChip, hexInput, resetBtn);

    // 6. Preset Swatches inside Popover (Monochrome & Accents)
    const presetsBox = document.createElement("div");
    presetsBox.className = "cpop-presets-box";

    const monoTitle = document.createElement("div");
    monoTitle.className = "cpop-preset-label";
    monoTitle.textContent = "基础黑白灰与透明度";
    const monoGrid = document.createElement("div");
    monoGrid.className = "cpop-swatch-grid";

    const accentTitle = document.createElement("div");
    accentTitle.className = "cpop-preset-label";
    accentTitle.textContent = "常用颜色";
    const accentGrid = document.createElement("div");
    accentGrid.className = "cpop-swatch-grid";

    const allButtons: Array<{ btn: HTMLButtonElement; hex8: string }> = [];

    const syncUiFromHsv = (hex8: string) => {
      svPlane.style.backgroundColor = `hsl(${Math.round(this.hsv.h)}, 100%, 50%)`;
      svThumb.style.left = `${this.hsv.s}%`;
      svThumb.style.top = `${100 - this.hsv.v}%`;
      svThumb.style.backgroundColor = rgbaCssFromHex8(hex8.slice(0, 7) + "FF");
      hueInput.value = String(Math.round(this.hsv.h));
      alphaInput.value = String(this.hsv.a);
      alphaValue.textContent = `${this.hsv.a}%`;
      previewChip.style.setProperty("--swatch-color", rgbaCssFromHex8(hex8));
      nativeColor.value = hex8.slice(0, 7);
      if (document.activeElement !== hexInput) {
        hexInput.value = hex8;
      }
      for (const item of allButtons) {
        item.btn.classList.toggle("active", item.hex8 === hex8);
      }
      // Also update mode switcher dot if present
      const activeModeDot = this.el.querySelector<HTMLElement>(".cpop-mode-btn.active .cpop-mode-dot");
      if (activeModeDot) {
        activeModeDot.style.background = rgbaCssFromHex8(hex8);
      }
    };

    const applyHsvChange = (explicitHex?: string) => {
      const nextHex = explicitHex ?? hsvaToHex8(this.hsv.h, this.hsv.s, this.hsv.v, this.hsv.a);
      syncUiFromHsv(nextHex);
      opts.onChange(this.currentMode, nextHex);
    };

    const buildSwatchBtn = (sw: ColorPopoverSwatch) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = `cpop-swatch-btn${sw.isTransparent ? " is-transparent" : ""}`;
      btn.title = sw.label;
      if (!sw.isTransparent) {
        btn.style.setProperty("--dot-color", rgbaCssFromHex8(sw.hex8));
      }
      btn.onclick = () => {
        this.hsv = hex8ToHsva(sw.hex8);
        applyHsvChange(sw.hex8);
      };
      allButtons.push({ btn, hex8: sw.hex8 });
      return btn;
    };

    for (const sw of MONOCHROME_SWATCHES) {
      monoGrid.append(buildSwatchBtn(sw));
    }
    for (const sw of ACCENT_SWATCHES) {
      accentGrid.append(buildSwatchBtn(sw));
    }

    presetsBox.append(monoTitle, monoGrid, accentTitle, accentGrid);

    // Bind 2D SV plane pointer drag
    svPlane.onpointerdown = (downEvent: PointerEvent) => {
      downEvent.preventDefault();
      svPlane.setPointerCapture(downEvent.pointerId);
      const updateFromPointer = (ev: PointerEvent) => {
        const rect = svPlane.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) return;
        this.hsv.s = clamp(((ev.clientX - rect.left) / rect.width) * 100, 0, 100);
        this.hsv.v = clamp(100 - ((ev.clientY - rect.top) / rect.height) * 100, 0, 100);
        if (this.hsv.a === 0) this.hsv.a = 100;
        applyHsvChange();
      };
      updateFromPointer(downEvent);
      svPlane.onpointermove = updateFromPointer;
      svPlane.onpointerup = (upEvent: PointerEvent) => {
        try {
          svPlane.releasePointerCapture(upEvent.pointerId);
        } catch {
          // Ignore
        }
        svPlane.onpointermove = null;
        svPlane.onpointerup = null;
      };
    };

    hueInput.oninput = () => {
      this.hsv.h = Number(hueInput.value);
      if (this.hsv.a === 0) this.hsv.a = 100;
      applyHsvChange();
    };

    alphaInput.oninput = () => {
      this.hsv.a = Number(alphaInput.value);
      applyHsvChange();
    };

    nativeColor.oninput = () => {
      const alphaHex = Math.round((this.hsv.a / 100) * 255)
        .toString(16)
        .toUpperCase()
        .padStart(2, "0");
      const nextHex = nativeColor.value.toUpperCase() + (this.hsv.a === 0 ? "FF" : alphaHex);
      this.hsv = hex8ToHsva(nextHex);
      applyHsvChange(nextHex);
    };

    hexInput.oninput = () => {
      let clean = hexInput.value.trim().toUpperCase();
      if (!clean.startsWith("#")) clean = "#" + clean;
      if (/^#[0-9A-F]{8}$/.test(clean)) {
        this.hsv = hex8ToHsva(clean);
        applyHsvChange(clean);
      }
    };

    hexInput.onchange = () => {
      let clean = hexInput.value.trim().toUpperCase();
      if (!clean.startsWith("#")) clean = "#" + clean;
      if (/^#[0-9A-F]{6}$/.test(clean)) clean += "FF";
      if (/^#[0-9A-F]{8}$/.test(clean)) {
        this.hsv = hex8ToHsva(clean);
        applyHsvChange(clean);
      } else {
        hexInput.value = opts.getHex(this.currentMode);
      }
    };

    syncUiFromHsv(currentHex);
    this.el.append(svPlane, slidersWrap, hexRow, presetsBox);
  }
}

let sharedPopover: StudioColorPopover | null = null;

export function openStudioColorPopover(options: ColorPopoverOpenOptions): void {
  if (typeof document === "undefined") return;
  if (!sharedPopover) {
    sharedPopover = new StudioColorPopover();
  }
  sharedPopover.open(options);
}

export function closeStudioColorPopover(): void {
  sharedPopover?.close();
}
