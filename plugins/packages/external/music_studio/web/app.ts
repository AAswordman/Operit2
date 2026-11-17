import { icon } from "./ui/icons";
import "./styles.css";
import { type Project, type Track, type Snapshot, type Operation, type Request, type Command, type PlaybackStatus, copy, uid, LIMITS } from "../src/shared/model";
import { PRESETS, EFFECTS } from "../src/shared/presets";
import { TEMPLATES, PATTERNS, generatePattern } from "../src/shared/composition";
import { SYNTH_RANGES, SYNTH_INTEGERS } from "../src/shared/validation";
import { WasmAudioEngine } from "./audio/wasm-engine";
import { renderWav } from "./audio/wasm-render";
import { request, saveFile, hostMode, base64 } from "./host";
import { studioLayout, escapeHtml, type LayoutState } from "./ui/layout";
import { trackRegions, regionNotes, regionWindow, type Region } from "./ui/regions";
import { drawTrack, drawPiano, drawScope, pitchBounds } from "./ui/canvas";
import { KnobControls } from "./ui/knobs";
import { gainLabel } from "./ui/mixer";
import { createLiveControl } from "./live-control";
import { auditionKeyboard, KeyboardAudition } from "./ui/audition-keyboard";
import { ViewNavigation, clampPiano, transformPiano, panPiano, transformTimeline, clamp, type PianoViewport, type Point, type ZoomAxis, type NavigationSurface } from "./ui/navigation";
const root = document.getElementById("app")!;
let snapshot: Snapshot;
let engine: WasmAudioEngine;
let selected = "";
let tab = "piano";
let category = "All";
let search = "";
let page = 0;
let visibleBars = innerWidth < 600 ? 1 : 4;
let mutationBusy = false;
let rendering = false;
let syncing = false;
let paintDirty = true;
let panel: LayoutState["panel"] = null;
let projectMenuOpen = false;
let editorOpen = false;
let selectedRegion: Region | null = null;
let pianoView: PianoViewport | null = null;
let previewOctave = 3;
let previewVelocity = .75;
let editorOrigin = "";
const timelineViewport = { left: 0, top: 0 };
let resetTimelineViewport = false;
let scopeOpen = false;
let timelineZoom = 0; // 0 = fitted song; otherwise lane width / (bars * 64).
const disclosures = new Set<string>();
let lastFrame = 0;
let lastCanvasBeat = -1;
let lastTrackPaint = 0;
let pollTimer: ReturnType<typeof setTimeout> | undefined;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
const processing = new Set<string>();
const activity: { time: string; text: string }[] = [];
const $ = <T extends HTMLElement = HTMLElement>(selector: string): T => root.querySelector<T>(selector)!;
const esc = escapeHtml;
const project = (): Project => snapshot.project;
const navigation = new ViewNavigation(root, resolveNavigation);
const knobs = new KnobControls(root);
const audition = new KeyboardAudition(root, {
  enabled: () => editorOpen && tab === "synth" && Boolean(track()) && !rendering,
  base: () => (previewOctave+1)*12, velocity: () => previewVelocity,
  on: (pitch,velocity) => engine.previewNoteOn(selected,pitch,velocity),
  off: pitch => engine?.previewNoteOff(pitch), clear: () => engine?.previewClear(),
  status(text,error = false) { const node=$("#preview-status"); if(node) { node.textContent=text; node.title=text; node.classList.toggle("error",error); } },
});
function refreshAudition(): void {
  audition.stopAll(); const panel=$(".audition-panel"); if(panel) panel.outerHTML=auditionKeyboard(previewOctave,previewVelocity);
}
function resolveNavigation(target: EventTarget | null): NavigationSurface | null {
  if (!(target instanceof Element)) return null;
  const piano = $<HTMLCanvasElement>("#piano");
  if (piano && target.closest("#piano,.piano-ruler")) return {
    element: piano, kind: "piano",
    pan(dx, dy) {
      if (!selectedRegion) return;
      const rect = piano.getBoundingClientRect();
      pianoView = panPiano(ensurePianoView(), selectedRegion, dx, dy, rect.width-42, rect.height);
      updatePianoChrome(); paintDirty = true;
    },
    zoom(scale, before, after, axis) {
      if (!selectedRegion) return;
      const rect = piano.getBoundingClientRect();
      const anchor = (point: Point): Point => ({ x: clamp((point.x-rect.left-42)/Math.max(1,rect.width-42),0,1), y: clamp((point.y-rect.top)/Math.max(1,rect.height),0,1) });
      pianoView = transformPiano(ensurePianoView(), selectedRegion, scale, anchor(before), anchor(after), axis);
      updatePianoChrome(); paintDirty = true;
    },
  };
  const timeline = $("#timeline-scroll");
  if (!timeline || !target.closest("#timeline-scroll") || target.closest(".track-info,.ruler-label,.section-label,[data-action],.empty-region")) return null;
  return { element: timeline, kind: "timeline", pan(dx,dy) { timeline.scrollLeft += dx; timeline.scrollTop += dy; saveTimelinePosition(); }, zoom: zoomTimeline };
}
function saveTimelinePosition(): void {
  const timeline = $("#timeline-scroll");
  if (timeline?.getClientRects().length) { timelineViewport.left = timeline.scrollLeft; timelineViewport.top = timeline.scrollTop; paintDirty = true; }
}
function zoomTimeline(scale: number, before: Point, after: Point, _axis: ZoomAxis = "time"): void {
  const timeline = $("#timeline-scroll"), ruler = $("#ruler"), grid = $(".timeline-grid"), label = $(".ruler-label");
  if (!timeline || !ruler || !grid || !label) return;
  const rect = timeline.getBoundingClientRect(), gutter = label.getBoundingClientRect().width;
  const fit = Math.max(1,timeline.clientWidth-gutter);
  const result = transformTimeline(ruler.getBoundingClientRect().width, timeline.scrollLeft, fit, Math.max(fit*16,project().bars*256), scale, clamp(before.x-rect.left-gutter,0,fit), clamp(after.x-rect.left-gutter,0,fit));
  timelineZoom = result.width <= fit+.5 ? 0 : result.width/(project().bars*64);
  grid.style.setProperty("--lane-min", `${timelineZoom === 0 ? 0 : result.width}px`);
  timeline.scrollLeft = result.left;
  timeline.scrollTop += before.y-after.y;
  const zoomLabel = $(".zoom-label"); if (zoomLabel) zoomLabel.textContent = timelineZoom === 0 ? "FIT" : `${Math.round(result.width/fit*100)}%`;
  for (let i=0;i<ruler.children.length;i++) ruler.children[i].textContent = result.width/project().bars < 25 && i%4 ? "" : String(i+1).padStart(2,"0");
  saveTimelinePosition();
}
function toolbarTimelineZoom(scale: number): void {
  const timeline = $("#timeline-scroll"), label = $(".ruler-label"); if (!timeline || !label) return;
  const rect = timeline.getBoundingClientRect();
  const point = { x: rect.left+(timeline.clientWidth+label.getBoundingClientRect().width)/2, y: rect.top };
  zoomTimeline(scale,point,point);
}
const track = (): Track | undefined => project().tracks.find(t => t.id === selected);
function revision(): { projectId: string; revision: number } { return { projectId: project().id, revision: project().revision }; }
function log(message: string): void { activity.unshift({ time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }), text: message }); activity.splice(6); renderActivity(); }
function notify(message: string, error = false): void {
  document.querySelector(".notice")?.remove(); if (noticeTimer) clearTimeout(noticeTimer);
  const div = document.createElement("div"); div.className = `notice${error ? " error" : ""}`; div.setAttribute("role", error ? "alert" : "status"); div.textContent = message; document.body.append(div); noticeTimer = setTimeout(() => div.remove(), error ? 11000 : 6000);
}
function accept(next: Snapshot, source = "工程已更新"): void {
  const changed = !snapshot || snapshot.project.id !== next.project.id || snapshot.project.revision !== next.project.revision;
  const switched = snapshot && snapshot.project.id !== next.project.id;
  snapshot = next;
  if (!changed) return;
  if (!engine) engine = new WasmAudioEngine(next.project); else engine.setProject(next.project);
  if (!next.project.tracks.some(t => t.id === selected)) selected = next.project.tracks[0]?.id ?? "";
  if (switched || (selectedRegion && !project().tracks.some(t => t.id === selectedRegion!.trackId))) {
    selectedRegion = null; pianoView = null; editorOpen = false; editorOrigin = ""; page = 0;
    if (switched) { timelineViewport.left = timelineViewport.top = 0; resetTimelineViewport = true; timelineZoom = 0; panel = null; }
  }
  if (selectedRegion) {
    selectedRegion.end = Math.min(selectedRegion.end, project().bars * project().beatsPerBar);
    if (selectedRegion.start >= selectedRegion.end) { selectedRegion = null; pianoView = null; editorOpen = false; }
    else if (pianoView) pianoView = clampPiano(pianoView, selectedRegion);
  }
  page = Math.min(page, project().bars - 1);
  log(source); render();
}
async function mutate(value: Request, message: string): Promise<void> {
  if (mutationBusy || rendering) { notify("正在处理，请稍候"); return; }
  mutationBusy = true;
  try { const next = await request({ ...value, ...revision() }); projectMenuOpen = false; accept(next, message); }
  catch (error) { notify(String(error), true); try { accept(await request({ action: "get" }), "已同步最新工程，请重试修改"); } catch { /* Preserve original error. */ } }
  finally { mutationBusy = false; }
}
function batch(operations: Operation[], message = "已自动保存"): Promise<void> { return mutate({ action: "batch", operations }, message); }
function currentTrackOp(type: string, extras: Record<string, unknown>): void { if (track()) void batch([{ type, trackId: selected, ...extras }]); }
function param(key: string, label: string, value: number, min: number, max: number, step: number, group = "synth", extra = ""): string {
  const id=esc(group+key+extra),logarithmic=min>0&&max/min>=100&&max>=100;
  return `<div class="parameter knob-parameter"><div class="knob-control"><svg viewBox="0 0 40 40" aria-hidden="true"><circle class="knob-face" cx="20" cy="20" r="16"/><path class="knob-arc" d="M8,32 A17,17 0 1,1 32,32"/><g class="knob-indicator"><path d="M20,5V9"/></g></svg><output for="${id}">${value>=1000?`${(value/1000).toFixed(1)}k`:Number(value.toFixed(3))}</output><input class="knob-input" id="${id}" aria-label="${esc(label)}" title="${esc(label)} · 上下拖动，Shift精调，方向键微调" type="range" min="${min}" max="${max}" step="${step}" value="${value}" data-knob-scale="${logarithmic?'log':'linear'}" data-param="${key}" data-group="${group}" data-effect="${extra}"></div><label for="${id}" title="${esc(label)}">${esc(label)}</label></div>`;
}
/** Rebuild only on discrete edits; preserve viewport/focus and disclosure state. */
function render(): void {
  navigation.cancel(); knobs.cancel(); audition.stopAll();
  const timeline = root.querySelector("#timeline-scroll");
  const editor = root.querySelector("#editor-body");
  const panelScroll = root.querySelector(".panel-scroll");
  if (!resetTimelineViewport && timeline?.getClientRects().length) { timelineViewport.left = timeline.scrollLeft; timelineViewport.top = timeline.scrollTop; }
  resetTimelineViewport = false;
  const scrolls = [["#timeline-scroll", timelineViewport.left, timelineViewport.top], ["#editor-body", editor?.scrollLeft ?? 0, editor?.scrollTop ?? 0], [".panel-scroll", panelScroll?.scrollLeft ?? 0, panelScroll?.scrollTop ?? 0]] as const;
  const focused = document.activeElement as HTMLElement | null;
  const focusId = focused?.id;
  const focusAction = focused?.dataset.action;
  root.innerHTML = studioLayout({ snapshot, selected, tab, editorOpen, region: selectedRegion, panel, menu: projectMenuOpen, category, search, scope: scopeOpen, zoom: timelineZoom, metronome: engine.metronome, host: hostMode() });
  renderPresets(); renderEditor(); renderActivity(); bind(); knobs.refresh(); restoreDisclosures();
  for (const [selector, left, top] of scrolls) { const node = root.querySelector(selector); if (node) { node.scrollLeft = left; node.scrollTop = top; } }
  if (focusId) root.querySelector<HTMLElement>(`#${CSS.escape(focusId)}`)?.focus({ preventScroll: true });
  else if (focusAction) root.querySelector<HTMLElement>(`[data-action="${CSS.escape(focusAction)}"]`)?.focus({ preventScroll: true });
  paintDirty = true; updateTransport();
}
function restoreDisclosures(): void {
  for (const details of root.querySelectorAll<HTMLDetailsElement>("details")) {
    const key = details.dataset.disclosure ?? details.querySelector("summary")?.textContent ?? "";
    details.open = disclosures.has(key);
    details.ontoggle = () => { if (details.open) disclosures.add(key); else disclosures.delete(key); paintDirty = true; };
  }
}
function setPanel(next: LayoutState["panel"]): void {
  panel = next; projectMenuOpen = false; if (next) editorOpen = false; render();
  if (next) $("#side-panel")?.focus({ preventScroll: true });
}
function openEditor(next: string): void {
  tab = next; editorOpen = true; projectMenuOpen = false; panel = null;
  render(); $("#editor-title")?.focus({ preventScroll: true });
}
function closeEditor(): void {
  editorOpen = false; render();
  const origin = editorOrigin ? root.querySelector<HTMLElement>(editorOrigin) : null;
  (origin ?? root.querySelector<HTMLElement>(`[data-select="${CSS.escape(selected)}"]`))?.focus({ preventScroll: true });
}
function openDevice(id: string, next = "synth"): void {
  selected = id; selectedRegion = null; pianoView = null;
  previewOctave = track()?.synth.engine === "drums" || track()?.preset.includes("bass") ? 2 : 3; editorOrigin = `[data-device="${CSS.escape(id)}"]`;
  openEditor(next); $("#editor-body").scrollTop = 0;
}
function openRegion(region: Region): void {
  selected = region.trackId; selectedRegion = { ...region }; pianoView = null; page = Math.floor(region.start / project().beatsPerBar);
  editorOrigin = `[data-region="${CSS.escape(region.key)}"]`;
  openEditor("piano"); $("#editor-body").scrollTop = 0;
}
function newRegion(id: string, beat = 0): void {
  const p = project(), t = p.tracks.find(t => t.id === id); if (!t) return;
  const regions = trackRegions(t, p), existing = regions.find(r => beat >= r.start && beat < r.end);
  if (existing) { openRegion(existing); return; }
  const before = regions.filter(r => r.end <= beat).pop(), after = regions.find(r => r.start > beat);
  const start = Math.max(before?.end ?? 0, Math.min((p.bars-1)*p.beatsPerBar, Math.floor(beat/p.beatsPerBar)*p.beatsPerBar));
  const end = Math.min(after?.start ?? p.bars*p.beatsPerBar, start+4*p.beatsPerBar);
  if (end <= start) return;
  openRegion({ key: `${id}:new:${start}`, trackId: id, name: "新乐段", start, end });
  editorOrigin = `[data-canvas="${CSS.escape(id)}"]`;
}
function editingTrack(): Track | undefined {
  const t = track(); return t && selectedRegion && selectedRegion.trackId === t.id ? { ...t, notes: regionNotes(t, selectedRegion) } : t;
}
function ensurePianoView(): PianoViewport {
  if (!pianoView) {
    const view = regionWindow(selectedRegion!,page,visibleBars,project().beatsPerBar);
    const [low,high] = pitchBounds(editingTrack()!);
    pianoView = clampPiano({ start: view.start, span: view.end-view.start, top: high+1, rows: high-low+1 }, selectedRegion!);
  }
  return pianoView;
}
function pianoWindow(): { start: number; end: number } {
  if (tab === "drums") return regionWindow(selectedRegion!,page,1,project().beatsPerBar);
  const view = ensurePianoView(); return { start: view.start, end: view.start+view.span };
}
function pianoRuler(): string {
  const view = pianoWindow(), bpb = project().beatsPerBar;
  return Array.from({ length: Math.ceil(view.end/bpb)-Math.floor(view.start/bpb) }, (_,i) => {
    const bar = Math.floor(view.start/bpb)+i, start = Math.max(view.start,bar*bpb), end = Math.min(view.end,(bar+1)*bpb);
    return `<span style="left:${(start-view.start)/(view.end-view.start)*100}%;width:${(end-start)/(view.end-view.start)*100}%">${bar+1}</span>`;
  }).join("");
}
function updatePianoChrome(): void {
  if (!selectedRegion || tab !== "piano") return;
  const view = ensurePianoView(); page = Math.floor(view.start/project().beatsPerBar);
  const range = $(".region-range"); if (range) range.textContent = `BAR ${page+1}–${Math.ceil((view.start+view.span)/project().beatsPerBar)}`;
  const prev = $<HTMLButtonElement>('[data-action="prev-bars"]'), next = $<HTMLButtonElement>('[data-action="next-bars"]');
  if (prev) prev.disabled = view.start <= selectedRegion.start+.0001;
  if (next) next.disabled = view.start+view.span >= selectedRegion.end-.0001;
  const zoom = $<HTMLSelectElement>("#zoom");
  if (zoom) {
    let free = zoom.querySelector<HTMLOptionElement>('option[value="free"]');
    if (!free) { free = document.createElement("option"); free.value = "free"; free.disabled = true; zoom.append(free); }
    free.textContent = `自由 · ${Number((view.span/project().beatsPerBar).toFixed(2))} 小节`;
    const preset = [1,2,4,8].find(n => Math.abs(n*project().beatsPerBar-view.span)<.0001);
    zoom.value = preset ? String(preset) : "free";
  }
  const ruler = $(".piano-ruler > div"); if (ruler) ruler.innerHTML = pianoRuler();
}
function movePage(direction: number, bars = visibleBars): void {
  if (!selectedRegion) return;
  if (tab === "piano") {
    const view = ensurePianoView(); pianoView = clampPiano({ ...view, start: view.start+direction*view.span }, selectedRegion);
    updatePianoChrome(); paintDirty = true; return;
  }
  const first = Math.floor(selectedRegion.start/project().beatsPerBar), last = Math.ceil(selectedRegion.end/project().beatsPerBar)-1;
  page = Math.max(first, Math.min(last, page + direction*bars)); renderEditor(); paintDirty = true;
}
function formatTime(seconds: number, millis = true): string { return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(Math.floor(seconds % 60)).padStart(2, "0")}${millis ? "." + String(Math.floor(seconds % 1 * 1000)).padStart(3, "0") : ""}`; }
function renderActivity(): void { const node = root.querySelector("#activity"); if (node) node.innerHTML = activity.map(a => `<div class="activity-entry"><time>${esc(a.time)}</time>${esc(a.text)}</div>`).join(""); }
function renderPresets(): void {
  const list = $("#presets"); if (!list) return;
  list.innerHTML = PRESETS.filter(p => (category === "All" || p.category === category) && (p.name + p.category + p.description).toLowerCase().includes(search.toLowerCase())).map(p => `<button class="preset" data-preset="${p.id}" title="${esc(p.description)}"><span class="preset-icon">${p.category === "Drums" ? "▦" : p.category === "Bass" ? "≋" : p.category === "Texture" ? "◌" : "∿"}</span><span><strong>${esc(p.name.split(" / ")[0])}</strong><small>${esc(p.name.split(" / ")[1])} · ${p.category}</small></span></button>`).join("");
}
function renderEditor(): void {
  const t = track(); const body = $("#editor-body"); if (!body) return;
  if (!editorOpen) return;
  if (tab === "settings") { renderSettings(); return; }
  if (tab === "help") {
    body.innerHTML = `<div class="help-panel"><strong style="color:var(--fg)">AI 是编曲师，这里是你的聆听空间。</strong><br>七组工具：<code>music_project / tracks / notes / instruments / effects / arrangement / transport</code><br>先调用 <code>music_project.get</code> 与 <code>catalog</code>，所有修改携带工程 ID 和 revision。<br>音符时间单位为拍（四分音符），音高用 MIDI 编号。<br>可让 AI：「做一段 A 小调、112 BPM 的电子音乐，增加渐进的鼓组与宽阔织体。」<br>编辑立即保存，AI 改动自动同步。播放需首次点击解锁音频；关闭页面后停止播放。<br>这是独立的纯合成工作台，不支持 VST、采样库、录音或硬件 MIDI。<br><button data-action="export-json">导出当前工程 JSON</button> <button data-action="import">导入工程</button> <button data-action="new">新建空白工程</button></div>`; return;
  }
  if (!t) { body.innerHTML = '<div class="fx-empty">添加或选择一条轨道，然后探索声音。</div>'; return; }
  if (tab === "piano") {
    if (!selectedRegion) return;
    const view = pianoWindow();
    const ruler = pianoRuler();
    body.innerHTML = `<div class="piano-toolbar"><span class="region-range">BAR ${Math.floor(view.start/project().beatsPerBar)+1}–${Math.ceil(view.end/project().beatsPerBar)}</span><button data-action="prev-bars" aria-label="前一组小节" ${view.start <= selectedRegion.start ? "disabled" : ""}>←</button><button data-action="next-bars" aria-label="后一组小节" ${view.end >= selectedRegion.end ? "disabled" : ""}>→</button><select id="zoom" aria-label="钢琴卷帘显示小节数">${[1, 2, 4, 8].map(n => `<option value="${n}" ${visibleBars === n ? "selected" : ""}>${n} 小节</option>`).join("")}</select><button data-action="region-device" class="region-device-link">∿ 乐器</button><div class="spacer"></div><span class="editing-hint">Ctrl/⌘ 滚轮缩放 · Shift 横移 · Alt 音高 · 双指缩放/移动</span><select id="pattern" aria-label="生成音型">${PATTERNS.map(k => `<option>${k}</option>`).join("")}</select><button data-action="generate">填充当前窗口</button></div><div class="piano-ruler" aria-label="当前乐段小节标尺"><span class="piano-ruler-gutter">BAR</span><div>${ruler}</div></div><canvas class="piano-canvas" id="piano" tabindex="0" aria-label="${esc(selectedRegion.name)} 钢琴卷帘，仅编辑此乐段内的音符"></canvas>`;
    updatePianoChrome();
  } else if (tab === "synth") {
    const s = t.synth; const names = { spectral: "SPECTRA / 双振荡器", fm: "ION / 双算子 FM", ensemble: "ATELIER / 合成乐器", drums: "CIRCUIT / 模拟鼓机", atmosphere: "AETHER / 立体声环境" };
    const labels: Record<string,string> = { blend:"A/B 混合",detune:"失谐 · ct",unison:"声部",width:"宽度",unisonB:"声部",detuneB:"失谐 · ct",widthB:"宽度",oscBOctave:"八度",oscBSemitone:"半音",oscBFine:"微调 · ct",subLevel:"Sub 电平",subOctave:"Sub 八度",noiseLevel:"噪声",phase:"起始相位",phaseRandom:"随机相位",haasMs:"延迟 · ms",haasMix:"干湿",bassMono:"低频收窄 · Hz",attack:"Attack",decay:"Decay",sustain:"Sustain",release:"Release",cutoff:"Cutoff · Hz",resonance:"Resonance",fmRatio:"比率",fmDepth:"深度",brightness:"明亮度",filterEnv:"Env · oct",lfoRate:"LFO · Hz",lfoDepth:"LFO · oct",pitchSweep:"滑音 · st" };
    const synthParam = (key: string): string => { const [min, max] = SYNTH_RANGES[key]; return param(key, labels[key], s[key as keyof typeof s] as number, min, max, SYNTH_INTEGERS.has(key) ? 1 : ["cutoff", "bassMono"].includes(key) ? 10 : ["detune", "detuneB", "oscBFine"].includes(key) ? 1 : key === "haasMs" ? .1 : .01); };
    const section = (title: string, keys: string[], help = ""): string => `<section class="synth-section"><h3>${title}</h3><div class="parameters">${keys.map(synthParam).join("")}</div>${help ? `<details class="synth-help" data-disclosure="help-${esc(title)}"><summary aria-label="${esc(title)} 说明" title="参数说明">ⓘ</summary><p>${help}</p></details>` : ""}</section>`;
    const dual = s.engine === "spectral" || s.engine === "ensemble", tonal = dual || s.engine === "fm";
    const waveNames: Record<string, string> = { sine: "正弦 / Sine", triangle: "三角 / Triangle", sawtooth: "锯齿 / Saw", square: "方波 / Square", glass: "玻璃谐波 / Glass", hollow: "空心谐波 / Hollow" };
    const waveSelect = (key: "wave" | "waveB", bank: string): string => {
      const w = s[key], points = Array.from({ length: 97 }, (_, i) => {
        const x = i / 96, a = 2 * Math.PI * x;
        const y = w === "sine" ? Math.sin(a) : w === "triangle" ? 1 - 4 * Math.abs(x - .5) : w === "sawtooth" ? 2*x-1 : w === "square" ? (x < .5 ? 1 : -1) : w === "glass" ? .72*Math.sin(a)+.16*Math.sin(3*a)+.08*Math.sin(7*a)+.04*Math.sin(11*a) : .85*Math.sin(a)+.11*Math.sin(3*a)+.04*Math.sin(5*a);
        return `${i ? "L" : "M"}${(x*192).toFixed(1)},${(24-y*19).toFixed(1)}`;
      }).join(" ");
      return `<div class="wave-control"><svg class="wave-preview" viewBox="0 0 192 48" aria-hidden="true"><path class="wave-axis" d="M0,24H192"/><path d="${points}"/></svg><label for="${key}">${bank} 波形</label><select id="${key}" data-synth-choice="${key}" aria-label="${bank} 波形">${Object.entries(waveNames).map(([value, name]) => `<option value="${value}" ${w === value ? "selected" : ""}>${name}</option>`).join("")}</select></div>`;
    };
    body.innerHTML = `<div class="synth-scroll"><div class="synth-panel"><div class="synth-top"><div class="synth-symbol">∿</div><div><div class="synth-name">${names[s.engine]}</div><div class="synth-sub">${esc(t.preset)} · PURE SYNTHESIS</div></div><div class="spacer"></div><select id="change-preset" aria-label="更换轨道音色">${PRESETS.map(p => `<option value="${p.id}" ${p.id === t.preset ? "selected" : ""}>${esc(p.name)}</option>`).join("")}</select></div>
    <div class="synth-engine"><label for="synth-engine">合成引擎</label><select id="synth-engine" data-synth-choice="engine" aria-label="合成引擎">${Object.entries(names).map(([value, name]) => `<option value="${value}" ${s.engine === value ? "selected" : ""}>${name}</option>`).join("")}</select></div>
    ${dual ? `<div class="oscillator-grid"><section class="oscillator-card"><h3>OSC A <small>${s.unison} VOICES · 主振荡器</small></h3>${waveSelect("wave", "A")}<div class="parameters">${["unison", "detune", "width"].map(synthParam).join("")}</div></section><section class="oscillator-card"><h3>OSC B <small>${s.unisonB} VOICES · 叠层</small></h3>${waveSelect("waveB", "B")}<div class="parameters">${["unisonB", "detuneB", "widthB", "oscBOctave", "oscBSemitone", "oscBFine"].map(synthParam).join("")}</div></section></div>` : ""}
    ${s.engine === "atmosphere" ? `<div class="air-description">分频气流 · 谐波泛音 · 缓慢漂移</div>` : ""}
    <div class="synth-modules">
    ${dual ? section("MIX · 相位", ["blend", "phase", "phaseRandom"], "相位随机量为0时固定起相；声部控制叠加振荡器数量。") : s.engine === "fm" ? section("FM · 双算子", ["fmRatio", "fmDepth", "phase"]) : s.engine === "drums" ? section("DRUM · 鼓音色", ["decay", "brightness"]) : s.engine === "atmosphere" ? `<section class="synth-section"><h3>AIR · 环境纹理</h3><div class="parameters">${param("brightness", "明亮度", s.brightness, 0, 1, .01)}${param("blend", "谐波比例", s.blend, 0, 1, .01)}${param("noiseLevel", "空气量", s.noiseLevel, 0, 1, .01)}${param("width", "立体声宽度", s.width, 0, 1, .01)}</div><details class="synth-help" data-disclosure="help-air"><summary aria-label="环境纹理说明" title="参数说明">ⓘ</summary><p>三段滤波气流缓慢漂移，谐波跟随当前音高；宽度为0时完全居中，不用延迟制造宽度。空气量与谐波比例可以分别调整。</p></details></section>` : ""}
    ${tonal ? section("SUB / NOISE", ["subLevel", "subOctave", "noiseLevel"], "SUB为居中正弦层；低八度可补厚Bass。") : ""}
    ${s.engine !== "drums" ? section("AMP · 包络", ["attack", "decay", "sustain", "release"]) + section("FILTER / LFO", ["cutoff", "resonance", "filterEnv", "lfoRate", "lfoDepth", "pitchSweep"]) : ""}
    ${section("STEREO / HAAS", ["haasMs", "haasMix", "bassMono"], "延迟正值=右声道，负值=左声道，0=关闭。低频收窄衰减侧信号；Haas折叠单声道可能改变音色。")}
    <section class="synth-section"><h3>OUTPUT · 输出</h3><div class="parameters">${param("gain", "音量", t.gain, 0, 1.5, 0.01, "track")}${param("pan", "声像", t.pan, -1, 1, 0.01, "track")}</div></section>
    </div>
    <details class="device-disclosure" data-disclosure="instrument-notes"><summary>关于此乐器</summary><p class="small muted">FM 为两算子；双振荡器与合成乐器支持每组 1–8 声部。弦乐等为振荡器合成近似音色，并非采样真实乐器。最大同时音符数仍为 64；叠加越多越耗 CPU。</p></details></div></div>${auditionKeyboard(previewOctave,previewVelocity)}`;
  } else if (tab === "effects") {
    body.innerHTML = `<div class="fx-list">${t.effects.map((fx, i) => `<div class="fx"><div class="fx-title"><span class="mono muted">0${i + 1}</span><strong>${EFFECTS[fx.type].name}</strong><span class="spacer"></span><button data-fx-toggle="${fx.id}" class="${fx.enabled ? "active" : ""}">${fx.enabled ? "ON" : "BYPASS"}</button><button data-fx-remove="${fx.id}" aria-label="移除效果器">×</button></div><details class="fx-details" data-disclosure="fx-${fx.id}"><summary>展开参数 / PARAMETERS</summary><div class="parameters">${param("mix", "DRY / WET", fx.mix, 0, 1, 0.01, "effect", fx.id)}${Object.entries(EFFECTS[fx.type].ranges).map(([key, [min, max]]) => param(key, key.toUpperCase(), fx.params[key], min, max, max > 100 ? 10 : max > 10 ? 0.1 : 0.001, "fxparam", fx.id)).join("")}</div></details></div>`).join("") || '<div class="fx-empty">干净的声音，留给你无限可能。<br><br>添加一个效果器，开始塑造空间与质感。</div>'}</div><div class="fx-add"><select id="effect-type" aria-label="效果器类型">${Object.entries(EFFECTS).map(([k, v]) => `<option value="${k}">${v.name}</option>`).join("")}</select><button data-action="add-effect">＋ 添加效果器</button></div>`;
  } else if (tab === "drums") {
    if (t.synth.engine !== "drums") { body.innerHTML = '<div class="fx-empty">先选择一条鼓轨，或添加一套模拟鼓组。<br><br><button data-preset="analog-kit">＋ Circuit 模拟鼓组</button></div>'; return; }
    if (!selectedRegion) return;
    const view = pianoWindow(); const start = view.start, bar = Math.floor(start/project().beatsPerBar);
    body.innerHTML = `<div class="piano-toolbar"><span>BAR ${bar + 1} · 前4拍 / 16 STEPS</span><button data-action="prev-drum" ${start <= selectedRegion.start ? "disabled" : ""}>←</button><button data-action="next-drum" ${view.end >= selectedRegion.end ? "disabled" : ""}>→</button><span class="spacer"></span><select id="pattern" aria-label="鼓组节奏类型">${["four-floor", "half-time", "breakbeat"].map(k => `<option>${k}</option>`).join("")}</select><button data-action="generate">填充当前小节</button></div><div class="drum-grid">${[[36, "KICK"], [38, "SNARE"], [39, "CLAP"], [42, "CLOSED HAT"], [46, "OPEN HAT"], [45, "TOM"], [49, "CRASH"], [56, "COWBELL"]].map(([pitch, name]) => `<div class="drum-row"><span>${name}</span>${Array.from({ length: 16 }, (_, step) => `<button class="drum-step ${t.notes.some(n => n.pitch === pitch && Math.abs(n.start - start - step / 4) < 0.01) ? "active" : ""}" data-drum="${pitch}" data-step="${step}" ${start + step / 4 >= view.end ? "disabled" : ""} aria-label="${name} 第${step + 1}步"></button>`).join("")}</div>`).join("")}</div>`;
  }
  knobs.refresh(); paintDirty = true;
}
function bind(): void {
  // Reassign properties to avoid accumulating event listeners when the shell is rebuilt.
  root.onclick = e => { if (navigation.consumeClick(e)) { e.preventDefault(); return; } const target = (e.target as HTMLElement).closest<HTMLElement>("button,[data-select],canvas,#ruler,[data-action]"); if (!target) return; void handleClick(target, e).catch(error => notify(String(error), true)); };
  const timeline = $("#timeline-scroll"); if (timeline) timeline.onscroll = saveTimelinePosition;
  root.oninput = e => {
    const input = e.target as HTMLInputElement;
    if (input.id === "preset-search") { search = input.value; renderPresets(); }
    if (input.type === "range") {
      const value=Number(input.value),gainId=input.dataset.trackGain??(input.dataset.group==="track"&&input.dataset.param==="gain"?selected:undefined);
      if(gainId) {
        for(const control of root.querySelectorAll<HTMLInputElement>(`[data-track-gain="${CSS.escape(gainId)}"]`)){control.value=String(value);control.setAttribute("aria-valuetext",gainLabel(value));}
        for(const output of root.querySelectorAll<HTMLOutputElement>(`[data-gain-output="${CSS.escape(gainId)}"]`))output.textContent=gainLabel(value);
        if(gainId===selected){const editorGain=root.querySelector<HTMLInputElement>('[data-group="track"][data-param="gain"]');if(editorGain){editorGain.value=String(value);const out=editorGain.parentElement?.querySelector("output");if(out)out.textContent=value.toFixed(2);}}
        engine.previewParameter(gainId,"track","gain",value);
      } else {
        const out=input.parentElement?.querySelector("output");if(out)out.textContent=value.toFixed(Number(input.step)<.01?3:2);
        if(input.dataset.group)engine.previewParameter(selected,input.dataset.group,input.dataset.param!,value,input.dataset.effect);
      }
    }
  };
  root.onchange = e => { void handleChange(e.target as HTMLInputElement).catch(error => notify(String(error), true)); };
  root.onkeydown = e => {
    const target = e.target as HTMLElement;
    if ((e.key === "Enter" || e.key === " ") && target.hasAttribute("data-select")) { e.preventDefault(); selected = target.dataset.select!; render(); }
    if (e.key === "Escape") { e.preventDefault(); if (projectMenuOpen) projectMenuOpen = false; else if (panel) panel = null; else { closeEditor(); return; } render(); }
    if (e.key === "Tab" && projectMenuOpen) {
      const dialog = root.querySelector(".project-menu")!;
      const focusable = [...dialog.querySelectorAll<HTMLElement>('button,select,input,[tabindex="0"]')];
      const first = focusable[0], last = focusable[focusable.length - 1];
      if (e.shiftKey && (document.activeElement === first || document.activeElement === dialog)) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    }
  };
}
async function handleChange(input: HTMLInputElement): Promise<void> {
  if (input.dataset.trackGain) {
    await batch([{type:"track.set",trackId:input.dataset.trackGain,patch:{gain:Number(input.value)}}],"保存分轨音量");
  }
  else if (input.id === "preview-velocity") { previewVelocity = Number(input.value); }
  else if (input.id === "projects") await mutate({ action: "open", id: input.value }, "切换工程");
  else if (input.id === "bpm") await batch([{ type: "project.set", patch: { bpm: Number(input.value) } }]);
  else if (input.id === "zoom" && input.value !== "free" && selectedRegion) {
    visibleBars = Number(input.value);
    pianoView = clampPiano({ ...ensurePianoView(), span: visibleBars*project().beatsPerBar }, selectedRegion);
    updatePianoChrome(); paintDirty = true;
  }
  else if (input.id === "section-jump") { const beat = Number(input.value); engine.seek(beat); page = Math.min(project().bars - 1, Math.floor(beat / project().beatsPerBar)); const timeline = $("#timeline-scroll"); const ruler = $("#ruler"); if (timeline && ruler) timeline.scrollLeft = beat / (project().bars * project().beatsPerBar) * ruler.clientWidth; paintDirty = true; updateTransport(); }
  else if (input.id === "change-preset") currentTrackOp("track.preset", { preset: input.value });
  else if (input.dataset.synthChoice) currentTrackOp("synth.set", { patch: { [input.dataset.synthChoice]: input.value } });
  else if (input.dataset.group) {
    const value = Number(input.value), key = input.dataset.param!; const group = input.dataset.group;
    if (group === "project") await batch([{ type: "project.set", patch: { [key]: value } }]);
    else if (group === "track") currentTrackOp("track.set", { patch: { [key]: value } });
    else if (group === "synth") currentTrackOp("synth.set", { patch: { [key]: value } });
    else currentTrackOp("effect.set", { effectId: input.dataset.effect, patch: group === "effect" ? { [key]: value } : { params: { [key]: value } } });
  } else if (input.id === "import-file" && input.files?.[0]) {
    const file = input.files[0]; if (file.size > LIMITS.jsonBytes) throw new Error("工程文件不能超过8MB"); await mutate({ action: "import", json: await file.text() }, "导入工程");
  }
}
async function handleClick(target: HTMLElement, event: MouseEvent): Promise<void> {
  const d = target.dataset;
  if (d.category) { category = d.category; render(); return; }
  if (d.tab) { tab = d.tab; render(); return; }
  if (d.device) { openDevice(d.device); return; }
  if (d.effects) { openDevice(d.effects, "effects"); return; }
  if (d.region) {
    const t = project().tracks.find(t => t.id === d.regionTrack);
    const region = t && trackRegions(t, project()).find(r => r.key === d.region);
    if (region) openRegion(region); return;
  }
  if (d.newRegion) { newRegion(d.newRegion); return; }
  if (d.select) { selected = d.select; render(); return; }
  if (d.preset) { await batch([{ type: "track.add", preset: d.preset }], "添加合成音色"); selected = project().tracks[project().tracks.length - 1]?.id ?? selected; panel = null; openDevice(selected); return; }
  if (d.template) { projectMenuOpen = false; panel = null; await mutate({ action: "create", template: d.template }, "从模板创建工程"); return; }
  if (d.mute || d.solo) { const id = d.mute || d.solo; const t = project().tracks.find(t => t.id === id)!; const key = d.mute ? "mute" : "solo"; await batch([{ type: "track.set", trackId: id, patch: { [key]: !t[key] } }]); return; }
  if (d.fxRemove) { currentTrackOp("effect.remove", { effectId: d.fxRemove }); return; }
  if (d.fxToggle) { const fx = track()!.effects.find(e => e.id === d.fxToggle)!; currentTrackOp("effect.set", { effectId: fx.id, patch: { enabled: !fx.enabled } }); return; }
  if (d.drum) {
    if (!selectedRegion) return;
    const t = editingTrack()!; const pitch = Number(d.drum), start = pianoWindow().start + Number(d.step) / 4;
    if (start >= pianoWindow().end) return;
    const exists = t.notes.find(n => n.pitch === pitch && Math.abs(n.start - start) < 0.01);
    currentTrackOp(exists ? "notes.remove" : "notes.add", exists ? { ids: [exists.id] } : { notes: [{ pitch, start, duration: Math.min(0.1, selectedRegion.end-start), velocity: 0.7 }] }); return;
  }
  if (target.id === "piano" && selectedRegion) {
    const t = editingTrack()!; const rect = target.getBoundingClientRect(); const x = event.clientX - rect.left; if (x < 42) return;
    const pv = ensurePianoView(); const pitch = clamp(Math.ceil(pv.top-(event.clientY-rect.top)/rect.height*pv.rows)-1,0,127);
    const view = pianoWindow();
    const start = Math.floor((view.start + (x - 42) / (rect.width - 42) * (view.end-view.start))*4)/4;
    if (start < selectedRegion.start || start >= view.end) return;
    const exists = t.notes.find(n => n.pitch === pitch && start >= n.start && start < n.start + n.duration);
    currentTrackOp(exists ? "notes.remove" : "notes.add", exists ? { ids: [exists.id] } : { notes: [{ pitch, start, duration: Math.min(t.synth.engine === "drums" ? 0.1 : 0.25, selectedRegion.end - start), velocity: 0.7 }] }); return;
  }
  if (d.canvas || target.id === "ruler") {
    const rect = (d.canvas ? target.parentElement! : target).getBoundingClientRect();
    const beat = rect.width ? (event.clientX-rect.left)/rect.width*project().bars*project().beatsPerBar : 0;
    if (d.canvas && event.detail >= 2) { newRegion(d.canvas, beat); return; }
    if (d.canvas) selected = d.canvas;
    engine.seek(beat); paintDirty = true; return;
  }
  switch (d.action) {
    case "play": if (engine.playing) engine.pause(); else await engine.play(); updateTransport(); break;
    case "stop": engine.stop(); updateTransport(); paintDirty = true; break;
    case "loop": await batch([{ type: "project.set", patch: { loop: { ...project().loop, enabled: !project().loop.enabled } } }]); break;
    case "metronome": engine.metronome = !engine.metronome; target.classList.toggle("active", engine.metronome); target.setAttribute("aria-pressed", String(engine.metronome)); break;
    case "library": setPanel(panel === "library" ? null : "library"); break;
    case "master": setPanel(panel === "master" ? null : "master"); break;
    case "close-panel": setPanel(null); break;
    case "project-menu": projectMenuOpen = !projectMenuOpen; panel = null; render(); if (projectMenuOpen) root.querySelector<HTMLElement>(".project-menu")?.focus(); break;
    case "close-menu": projectMenuOpen = false; render(); break;
    case "add-track": setPanel("library"); $("#preset-search").focus(); break;
    case "close-editor": closeEditor(); break;
    case "help": editorOrigin = '[data-action="help"]'; openEditor("help"); break;
    case "region-device": openDevice(selected); break;
    case "scope": scopeOpen = !scopeOpen; render(); break;
    case "zoom-in": toolbarTimelineZoom(1.5); break;
    case "zoom-out": toolbarTimelineZoom(1/1.5); break;
    case "zoom-fit": toolbarTimelineZoom(.00001); break;
    case "preview-octave-down": previewOctave = Math.max(0,previewOctave-1); refreshAudition(); break;
    case "preview-octave-up": previewOctave = Math.min(7,previewOctave+1); refreshAudition(); break;
    case "prev-bars": movePage(-1); break;
    case "next-bars": movePage(1); break;
    case "prev-drum": movePage(-1, 1); break;
    case "next-drum": movePage(1, 1); break;
    case "add-effect": currentTrackOp("effect.add", { effectType: $<HTMLSelectElement>("#effect-type").value }); break;
    case "generate": {
      if (!track() || !selectedRegion) break;
      const view = pianoWindow(), p = project();
      const notes = generatePattern({ kind: $<HTMLSelectElement>("#pattern").value, root: track()!.preset.includes("bass") ? 45 : 57, start: view.start, bars: Math.ceil((view.end-view.start)/p.beatsPerBar), seed: 1, scale: "minor", velocity: .7 }, p.beatsPerBar)
        .filter(n => n.start < view.end).map(n => ({ ...n, duration: Math.min(n.duration, view.end-n.start) })).filter(n => n.duration >= .01);
      const ids = track()!.notes.filter(n => n.start >= view.start && n.start < view.end).map(n => n.id);
      await batch([...(ids.length ? [{ type: "notes.remove", trackId: selected, ids }] : []), { type: "notes.add", trackId: selected, notes }], "填充当前乐段窗口"); break;
    }
    case "undo": case "redo": await mutate({ action: d.action }, d.action === "undo" ? "撤销修改" : "重做修改"); break;
    case "new": await mutate({ action: "create", name: "Untitled session" }, "新建空白工程"); break;
    case "import": $<HTMLInputElement>("#import-file").click(); break;
    case "export-json": { const p = project(); const path = await saveFile(`${p.id}_${p.revision}.operitmusic.json`, new TextEncoder().encode(JSON.stringify(p, null, 2))); notify(`工程已导出：${path}`); break; }
    case "render": await exportAudio(); break;
    case "settings": showSettings(); break;
    case "save-settings": {
      const name = $<HTMLInputElement>("#setting-name").value; const bpm = Number($<HTMLInputElement>("#setting-bpm").value); const bars = Number($<HTMLInputElement>("#setting-bars").value); const beatsPerBar = Number($<HTMLInputElement>("#setting-beats").value); const swing = Number($<HTMLInputElement>("#setting-swing").value);
      await batch([{ type: "project.set", patch: { name, bpm, bars, beatsPerBar, swing, loop: { enabled: project().loop.enabled, start: 0, end: bars * beatsPerBar } } }], "更新工程设置"); break;
    }
    case "duplicate": await mutate({ action: "duplicate" }, "复制工程"); break;
    case "remove-track": if (track() && confirm(`移除轨道「${track()!.name}」及其所有音符？`)) currentTrackOp("track.remove", {}); break;
    case "delete-project": if (confirm(`永久删除工程「${project().name}」？此操作不能撤销。`)) await mutate({ action: "delete", confirm: project().id }, "已删除工程"); break;
  }
}
function showSettings(): void { editorOrigin = '[data-action="project-menu"]'; openEditor("settings"); }
function renderSettings(): void {
  const p = project();
  $("#editor-body").innerHTML = `<div class="synth-panel"><div class="synth-top"><div class="synth-name">工程 / SESSION</div><span class="spacer"></span><span class="small muted">${esc(p.id)}</span></div><div class="parameters">${[["name", "工程名称", p.name, "text"], ["bpm", "BPM", p.bpm, "number"], ["bars", "小节数", p.bars, "number"], ["beats", "每小节拍数", p.beatsPerBar, "number"], ["swing", "Swing 0–0.65", p.swing, "number"]].map(([key, name, value, type]) => `<div class="parameter"><label for="setting-${key}">${name}</label><input style="width:100%;margin-top:8px" id="setting-${key}" type="${type}" ${key === "swing" ? 'step="0.01"' : ""} value="${esc(value)}"></div>`).join("")}</div><p class="small muted">缩短工程时，越界音符或段落会导致校验失败，不会静默裁剪。请先通过 AI 编辑处理。</p><div class="row wrap"><button class="primary" data-action="save-settings">保存设置</button><button data-action="duplicate">复制工程</button><button data-action="new">新建工程</button><button data-action="import">导入</button><button data-action="remove-track">移除所选轨道</button><button data-action="delete-project">删除工程</button></div></div>`;
}
function updateTransport(): void {
  if (!engine || !snapshot) return;
  const p = project(), beat = engine.beat; const button = $("#play");
  if (button && button.dataset.playing !== String(engine.playing)) {
    button.innerHTML = engine.playing ? '<span aria-hidden="true">Ⅱ</span>' : icon("play");
    button.dataset.playing = String(engine.playing);
  }
  const clock = $("#clock"); if (clock) clock.textContent = `${String(Math.floor(beat / p.beatsPerBar) + 1).padStart(3, "0")} : ${String(Math.floor(beat % p.beatsPerBar) + 1).padStart(2, "0")}`;
  const time = $("#time"); if (time) time.textContent = `${formatTime(beat * 60 / p.bpm)} / ${formatTime(p.bars * p.beatsPerBar * 60 / p.bpm, false)}`;
}
function frame(now: number): void {
  requestAnimationFrame(frame); if (!engine || !snapshot || document.hidden || now - lastFrame < 33) return; lastFrame = now;
  updateTransport(); const peak = engine.peak(); const transportMeter = $("#transport-meter"); if (transportMeter) transportMeter.style.height = `${Math.min(100, peak * 140)}%`; const db = peak > 0.00001 ? Math.max(-60, 20 * Math.log10(peak)) : -60;
  const stereo = engine.stereoPeak();
  ["meter-left", "meter-right"].forEach((key, i) => { const el = $("#" + key); const channelDb = 20 * Math.log10(Math.max(0.000001, stereo[i])); if (el) el.style.height = `${Math.max(0, Math.min(100, (60 + channelDb) / 60 * 100))}%`; });
  const scopeDb = $("#scope-db"); if (scopeDb) scopeDb.textContent = `${peak > 0.00001 ? db.toFixed(1) : "−∞"} dB`;
  const d = $("#master-db"); if (d) d.textContent = peak > 0.00001 ? db.toFixed(1) : "−∞";
  const voices = $("#voices"); if (voices) voices.textContent = `${engine.activeVoices} / ${LIMITS.voices}`;
  const rate = $("#sample-rate"); if (rate) rate.textContent = engine.context ? `${(engine.context.sampleRate / 1000).toFixed(1)} kHz` : "—";
  const dropped = $("#dropped"); if (dropped) dropped.textContent = String(engine.dropped);
  for (const node of root.querySelectorAll<HTMLElement>("[data-meter]")) node.style.width = `${Math.min(100, engine.peak(node.dataset.meter) * 180)}%`;
  const scope = $<HTMLCanvasElement>("#overview-scope"); if (scope) drawScope(scope, engine, "waveform");
  const spectrum = $<HTMLCanvasElement>("#spectrum"); if (spectrum) drawScope(spectrum, engine, "spectrum");
  const beat = engine.beat;
  if (paintDirty || (beat !== lastCanvasBeat && now - lastTrackPaint > 50)) {
    const timeline = $("#timeline-scroll"), viewport = timeline?.getBoundingClientRect();
    const gutter = $(".ruler-label")?.getBoundingClientRect().width ?? 0;
    if (timeline && viewport && viewport.height > 0) for (const canvas of root.querySelectorAll<HTMLCanvasElement>("[data-canvas]")) {
      const lane = canvas.parentElement!, rect = lane.getBoundingClientRect();
      if (rect.bottom < viewport.top || rect.top > viewport.bottom) continue;
      const offset = clamp(viewport.left+gutter-rect.left,0,rect.width), width = Math.min(rect.width-offset,Math.max(1,timeline.clientWidth-gutter));
      canvas.style.position = "absolute"; canvas.style.left = `${offset}px`; canvas.style.width = `${width}px`;
      const t = project().tracks.find(t => t.id === canvas.dataset.canvas);
      if (t) { const total = project().bars*project().beatsPerBar; drawTrack(canvas,t,project(),beat,offset/rect.width*total,(offset+width)/rect.width*total); }
    }
    const piano = $<HTMLCanvasElement>("#piano"); if (piano && selectedRegion && editingTrack()) { const view = pianoWindow(); drawPiano(piano, editingTrack()!, project(), beat, view.start, view.end, ensurePianoView()); }
    paintDirty = false; lastCanvasBeat = beat; lastTrackPaint = now;
  }
}
function status(): PlaybackStatus { return { connected: true, unlocked: engine.unlocked, playing: engine.playing, beat: engine.beat, peak: engine.peak(), voices: engine.activeVoices, dropped: engine.dropped, updatedAt: Date.now(), projectId: project().id }; }
async function command(c: Command): Promise<void> {
  if (processing.has(c.id)) return; processing.add(c.id);
  try {
    if (c.projectId !== project().id || c.revision !== project().revision) throw new Error("Command revision changed; not executed");
    let path: string | undefined;
    if (c.type === "play") { if (!engine.unlocked) throw new Error("请用户先点击一次播放，解锁音频"); await engine.play(); }
    else if (c.type === "pause") engine.pause();
    else if (c.type === "stop") engine.stop();
    else if (c.type === "seek") engine.seek(c.beat!);
    else path = await exportAudio(c);
    await request({ action: "ack", id: c.id, state: "done", message: c.type === "render" ? "WAV rendered and saved" : `Applied ${c.type}`, ...(path ? { path } : {}) });
    log(`AI · ${c.type === "render" ? "完成渲染" : c.type}`);
  } catch (error) {
    try { await request({ action: "ack", id: c.id, state: "failed", message: String(error).slice(0, 900) }); } catch { /* Project switch already cancelled the command. */ }
    notify(String(error), true);
  } finally { updateTransport(); paintDirty = true; processing.delete(c.id); }
}
async function sync(): Promise<void> {
  if (syncing || !engine) return; syncing = true;
  try {
    const next = await request<Partial<Snapshot>>({ action: "sync", ...revision(), status: status() });
    if (next.project && next.projects) accept(next as Snapshot, "AI / 外部编辑已同步");
    const node = $("#transport-status"); if (node) node.textContent = `${hostMode() ? "AI 已连接" : "浏览器预览"} · 工程已自动保存\n${engine.unlocked ? "音频就绪" : "点击播放启用音频"}`;
    // Do not await a render here: heartbeat must continue while native offline rendering runs.
    for (const c of next.commands ?? []) if (!processing.has(c.id)) void command(c);
  } catch (error) { const node = $("#transport-status"); if (node) node.textContent = `连接异常：${String(error).slice(0, 90)}`; }
  finally { syncing = false; }
}
async function exportAudio(command?: Command): Promise<string> {
  if (rendering) throw new Error("已有渲染任务进行中"); rendering = true; engine.pause(); audition.stopAll(); const p = copy(project());
  const overlay = document.createElement("div"); overlay.className = "render-overlay"; overlay.setAttribute("role", "status"); overlay.innerHTML = '<div class="render-spinner"></div><h2>把灵感变成声音文件。</h2><div class="muted" id="render-progress">准备离线渲染…</div><div class="small muted">本地合成 · 不上传任何音频</div>'; document.body.append(overlay);
  try {
    const result = await renderWav(p, message => { overlay.querySelector("#render-progress")!.textContent = message; });
    if (command) { const current = await request({ action: "get" }); if (current.project.id !== command.projectId || current.project.revision !== command.revision || !current.commands.some(c => c.id === command.id)) throw new Error("工程在渲染期间被修改，已取消导出；请重新渲染"); }
    const path = await saveFile(`${p.id}_r${p.revision}_${Date.now()}.wav`, result.bytes);
    log(`WAV · ${(result.bytes.length / 1048576).toFixed(2)} MB · ${result.dropped ? `丢弃${result.dropped}个超限音符` : "完成"}`);
    notify(`WAV 已保存：${path}${result.dropped ? `（${result.dropped} 个音符超过声部上限）` : ""}`); return path;
  } finally { rendering = false; overlay.remove(); updateTransport(); }
}
async function initialize(): Promise<void> {
  root.innerHTML = '<div class="welcome"><strong>operit / music studio</strong><span>正在唤醒合成器…</span></div>';
  try {
    let next: Snapshot | undefined; let lastError: unknown;
    for (let attempt = 0; attempt < 8; attempt++) { try { next = await request({ action: "get" }); break; } catch (e) { lastError = e; await new Promise(resolve => setTimeout(resolve, 400)); } }
    if (!next) throw lastError; accept(next, "工程就绪 · 所有音色均为本地合成");
    const schedulePoll = async (): Promise<void> => { await sync(); pollTimer = setTimeout(schedulePoll, document.hidden ? 2000 : 700); }; void schedulePoll();
    engine.onEnded = updateTransport;
    window.addEventListener("resize", () => { paintDirty = true; });
    new ResizeObserver(() => { paintDirty = true; }).observe(root);
    document.addEventListener("visibilitychange", () => { if (document.hidden) audition.stopAll(); if (document.hidden && engine.playing) { engine.pause(); log("页面进入后台，已暂停以避免节拍漂移"); } paintDirty = true; });
    window.addEventListener("pagehide", () => { audition.stopAll(); engine.stop(); if (pollTimer) clearTimeout(pollTimer); void engine.dispose(); });
    window.addEventListener("keydown", e => { if (e.defaultPrevented) return; if (e.key === "Escape") { if (projectMenuOpen) projectMenuOpen = false; else if (panel) panel = null; else { closeEditor(); return; } render(); return; } if (["INPUT", "SELECT", "TEXTAREA", "BUTTON"].includes((e.target as HTMLElement).tagName) || e.ctrlKey || e.metaKey || rendering) return; if (e.code === "Space") { e.preventDefault(); if (engine.playing) engine.pause(); else void engine.play().catch(e => notify(String(e), true)); } });
    // Opt-in direct control of this exact preview. No second project copy or network RPC server.
    if (new URLSearchParams(location.search).has("control") && /^(localhost|127\.0\.0\.1|\[::1\])$/.test(location.hostname) && !hostMode()) {
      window.musicStudioControl = createLiveControl({
        async read() { const next = await request<Snapshot>({ action:"get" }); accept(next,"同步当前网页工程"); return copy(project()); },
        async write(value,label) {
          if (mutationBusy || rendering) throw Error("BUSY: 网页正在保存或渲染，请等待后重新读取工程");
          mutationBusy = true;
          try { const next = await request<Snapshot>(value); accept(next,label); return copy(next.project); }
          finally { mutationBusy = false; }
        },
        status,
        async transport(type,beat) {
          if (type === "play") await engine.play();
          else if (type === "pause") engine.pause();
          else if (type === "stop") { audition.stopAll(); engine.stop(); }
          else engine.seek(beat!);
          paintDirty = true; updateTransport();
        },
        view(type,id,beat) {
          if (type === "arrangement") { panel=null; projectMenuOpen=false; closeEditor(); }
          else if (type === "piano") newRegion(id!,beat);
          else openDevice(id!,type === "effects" ? "effects" : "synth");
        },
        catalog: () => request({ action:"catalog" }),
      });
      log("协作接口已启用 · 等待试听指导，不自动改编曲");
    }
    // Diagnostics are opt-in on a local browser preview, never exposed to packaged production pages.
    if (new URLSearchParams(location.search).has("test") && !hostMode()) window.__musicTest = {
      audition: () => ({ pitches: audition.pitches, enginePitches: engine.auditionPitches, octave: previewOctave, velocity: previewVelocity }), project: () => copy(project()), editor: () => ({ open: editorOpen, tab, selected, region: copy(selectedRegion), page, visibleBars, viewport: pianoView ? { ...pianoView } : null }), navigation: () => ({ zoom: timelineZoom, ...timelineViewport, width: $("#ruler")?.getBoundingClientRect().width ?? 0, fit: $("#timeline-scroll") ? $("#timeline-scroll").clientWidth-$(".ruler-label").getBoundingClientRect().width : 0 }), snapshot: () => request({ action: "get" }), request: value => request(value),
      render: async (includeAudio = false) => { const r = await renderWav(project()); return { audio: includeAudio ? base64(r.bytes) : undefined, peak: r.peak, bytes: r.bytes.length, dropped: r.dropped, duration: r.duration, analysis: r.analysis }; },
      play: () => engine.play(), stop: () => engine.stop(), state: () => ({ playing: engine.playing, beat: engine.beat, voices: engine.activeVoices, peak: engine.peak(), stereo: engine.stereoPeak(), dropped: engine.dropped, state: engine.context?.state, dsp: engine.diagnostics() }),
    };
    requestAnimationFrame(frame);
  } catch (error) { root.innerHTML = `<div class="welcome"><strong>无法打开音乐工作台</strong><span class="loading-error">${esc(error)}</span><button id="retry">重试连接</button><span>已有工程不会被覆盖。</span></div>`; document.getElementById("retry")!.onclick = () => void initialize(); }
}
void initialize();
