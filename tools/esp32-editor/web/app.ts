import {setupEditor} from './editor.js';
import {request} from './transport.js';
import {point, pixel} from './model.js';
import {errorMessage, query} from './types.js';
import type {BoardInfo, BuildManifest, BuildStatus, RuntimeFactory, RuntimeModule} from './types.js';

/** Returns the 2D drawing context for a canvas. */
function drawingContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const context = canvas.getContext('2d');
  if (!context) throw new Error('无法创建屏幕绘制上下文');
  return context;
}

const screen = query<HTMLCanvasElement>('#screen');
const renderContext = drawingContext(screen);

const pageLabel = query<HTMLElement>('#page');
const eventList = query<HTMLOListElement>('#events');
const fpsSelect = query<HTMLSelectElement>('#fps');
const colorSelect = query<HTMLSelectElement>('#color');
const themeSelect = query<HTMLSelectElement>('#theme');
const shapeSelect = query<HTMLSelectElement>('#shape');
const wifiInput = query<HTMLInputElement>('#wifi');
const edgeInput = query<HTMLInputElement>('#edge');
const expressionSelect = query<HTMLSelectElement>('#expression');
const gridInput = query<HTMLInputElement>('#show-grid');
const grid = query<HTMLElement>('#grid');
const buildStatus = query<HTMLElement>('#build-status');
const editorStatus = query<HTMLElement>('#editor-status');
const sourceLabel = query<HTMLElement>('#source');
const coordinateLabel = query<HTMLElement>('#coords');
const renderTimeLabel = query<HTMLElement>('#render-time');
const stage = query<HTMLElement>('.stage');
const device = query<HTMLElement>('.device');
const zoomSelect = query<HTMLSelectElement>('#zoom');
const canvas = screen;

let runtime: RuntimeModule | null = null;
let board: BoardInfo | null = null;
let generation = -1;
let lastFrame = 0;
let pressed = false;
let sourceHash = '';
interface DeviceState {
  running?: boolean; connected?: boolean; pairingCode?: string; spaceState?: string;
  chatPreview?: string; chatScreen?: string; chatTask?: string;
  chatSendResult?: {ok: boolean; error?: string} | null;
}
let deviceRunning = false;
const composer = document.createElement('form');
composer.className = 'host-composer';
composer.innerHTML = '<label>电脑键盘输入（支持中文输入法）<input name="message" maxlength="120" autocomplete="off" placeholder="输入后发送到屏幕中的当前对话"></label><button type="submit">发送</button><output aria-live="polite"></output>';
stage.after(composer);
const hostInput = composer.querySelector<HTMLInputElement>('input')!;
const hostStatus = composer.querySelector<HTMLOutputElement>('output')!;
let submittedHostText = '';
composer.addEventListener('submit', event => {
  event.preventDefault();
  if (!runtime || window.operitEditor?.isEditing()) return;
  submittedHostText = hostInput.value;
  runtime.ccall('operit_lvgl_set_chat_draft', null, ['string'], [hostInput.value]);
  runtime.ccall('operit_lvgl_submit_chat', null, [], []);
});
window.addEventListener('operit-simulator-state', event => {
  const state = (event as CustomEvent<DeviceState>).detail;
  wifiInput.disabled = !!state.running;
  edgeInput.disabled = !!state.running;
  if (state.running) {
    wifiInput.checked = true;
    edgeInput.checked = !!state.connected;
    if (runtime) {
      applyControls();
      setDeviceState(state);
    }
  } else if (deviceRunning && runtime) {
    wifiInput.checked = edgeInput.checked = false;
    applyControls();
    setDeviceState({chatTask: '离线', chatScreen: '模拟设备已停止', chatSendResult: {ok: false, error: '模拟设备已停止，草稿已保留'}});
  }
  deviceRunning = !!state.running;
});

/** Adds a timestamped message to the bounded event log. */
function log(message: string): void {
  const item = document.createElement('li');
  const time = document.createElement('time');
  time.textContent = new Date().toLocaleTimeString();
  item.append(time, document.createTextNode(message));
  eventList.prepend(item);
  while (eventList.children.length > 80) {
    eventList.lastElementChild?.remove();
  }
}

/** Returns the initialized WebAssembly runtime. */
function activeRuntime(): RuntimeModule {
  if (!runtime) throw new Error('LVGL WebAssembly 尚未初始化');
  return runtime;
}

/** Returns the board metadata loaded from the editor service. */
function activeBoard(): BoardInfo {
  if (!board) throw new Error('开发板信息尚未加载');
  return board;
}

/** Returns a build manifest with the required source hash. */
function validManifest(manifest: BuildManifest): Required<Pick<BuildManifest, 'sourceHash'>> & BuildManifest {
  if (typeof manifest.sourceHash !== 'string' || !manifest.sourceHash) {
    throw new Error('构建清单缺少 sourceHash');
  }
  return manifest as Required<Pick<BuildManifest, 'sourceHash'>> & BuildManifest;
}

/** Updates the visible board theme and color swatches from LVGL state. */
function syncTheme(): void {
  const currentRuntime = activeRuntime();
  const currentBoard = activeBoard();
  const index = currentRuntime._operit_lvgl_theme_index();
  const theme = currentBoard.themes[index];
  if (!theme) throw new Error('LVGL 返回了不存在的主题索引');
  themeSelect.value = String(index);
  shapeSelect.value = currentRuntime._operit_lvgl_round_icons() ? 'circle' : 'square';
  for (const key of ['bg', 'surface', 'accent', 'muted'] as const) {
    const swatch = query<HTMLElement>('#swatches').querySelector<HTMLElement>(`[data-color="${key}"]`);
    if (swatch) swatch.style.backgroundColor = theme[key];
  }
}

/** Applies the debug panel state to the shared LVGL runtime. */
function applyControls(): void {
  const currentRuntime = activeRuntime();
  currentRuntime._operit_lvgl_set_connection(wifiInput.checked, edgeInput.checked);
  currentRuntime.ccall('operit_lvgl_set_expression', null, ['string'], [expressionSelect.value]);
}

function setDeviceState(state: DeviceState): void {
  if (!runtime) return;
  runtime.ccall('operit_lvgl_set_pairing_code', null, ['string'], [state.pairingCode ?? '']);
  runtime.ccall('operit_lvgl_set_space_state', null, ['string'], [state.spaceState ?? '等待连接 Operit']);
  runtime.ccall('operit_lvgl_set_chat_preview', null, ['string'], [state.chatPreview ?? '尚未连接对话']);
  runtime.ccall('operit_lvgl_set_chat_screen', null, ['string'], [state.chatScreen ?? '连接 Operit 后开始聊天']);
  runtime.ccall('operit_lvgl_set_chat_task', null, ['string'], [state.chatTask ?? '离线']);
  if (state.chatSendResult) finishSend(state.chatSendResult.ok, state.chatSendResult.error);
}

function finishSend(ok: boolean, error = ''): void {
  runtime?.ccall('operit_lvgl_chat_send_result', null, ['number', 'string'], [ok ? 1 : 0, error]);
  hostStatus.textContent = ok ? '已发送' : error;
  if (ok && hostInput.value === submittedHostText) hostInput.value = '';
}

/** Converts the Wasm RGB565 framebuffer into the visible Canvas image. */
function renderFrame(now: number): void {
  const currentRuntime = runtime;
  const interval = 1000 / Number(fpsSelect.value);
  if (currentRuntime && now - lastFrame >= interval) {
    lastFrame = now;
    const started = performance.now();
    currentRuntime._operit_lvgl_pump(0);
    const nextGeneration = currentRuntime._simulator_generation();
    if (nextGeneration !== generation) {
      generation = nextGeneration;
      const image = renderContext.createImageData(320, 240);
      const pointer = currentRuntime._simulator_frame();
      const pixels = new Uint16Array(currentRuntime.HEAPU8.buffer, pointer, 320 * 240);
      for (let index = 0; index < pixels.length; index += 1) {
        const value = pixels[index];
        const red = Math.round((value >> 11) * 255 / 31);
        const green = Math.round(((value >> 5) & 63) * 255 / 63);
        const blue = Math.round((value & 31) * 255 / 31);
        const rgb = pixel(red, green, blue, colorSelect.value as 'rgb565' | 'rgb332');
        image.data.set([rgb[0], rgb[1], rgb[2], 255], index * 4);
      }
      renderContext.putImageData(image, 0, 0);
    }
    syncTheme();
    pageLabel.textContent = currentRuntime.ccall('operit_lvgl_current_page', 'string', [], []);
    renderTimeLabel.textContent =
      `${(performance.now() - started).toFixed(1)} ms · LVGL ${Math.round(currentRuntime._simulator_heap_used() / 1024)} KiB`;
  }
  requestAnimationFrame(renderFrame);
}

/** Sends a logical touch event to the shared LVGL runtime. */
function touch(clientX: number, clientY: number, down: boolean): void {
  if (!runtime) return;
  const position = point(clientX, clientY, canvas.getBoundingClientRect());
  coordinateLabel.textContent = `${position.x},${position.y}`;
  runtime._simulator_touch(position.x, position.y, down ? 1 : 0);
}

/** Navigates the runtime to the home page or the application page. */
function navigate(page: 'home' | 'apps'): void {
  window.dispatchEvent(new Event('operit-runtime-navigation'));
  const currentRuntime = activeRuntime();
  if (page === 'home') {
    currentRuntime._operit_lvgl_navigate_home();
    pageLabel.textContent = 'Home';
    return;
  }
  currentRuntime._operit_lvgl_navigate_apps();
  pageLabel.textContent = 'Apps';
}

/** Handles runtime keyboard navigation outside edit mode. */
function handleCanvasKeydown(event: KeyboardEvent): void {
  if (window.operitEditor?.isEditing()) return;
  if (!['ArrowRight', 'ArrowLeft', 'Escape'].includes(event.key)) return;
  event.preventDefault();
  navigate(event.key === 'ArrowRight' ? 'apps' : 'home');
}

/** Handles action callbacks emitted by the shared LVGL runtime. */
function handleRuntimeAction(value: string): void {
  if (value.startsWith('navigate:')) {
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent<string>('operit-navigate-page', {detail: value.slice(9)}));
    }, 0);
    return;
  }
  log('LVGL action: ' + value);
  if (value === 'edge_send') {
    const text = activeRuntime().ccall('operit_lvgl_chat_draft', 'string', [], []);
    hostStatus.textContent = '发送中';
    void fetch('/api/simulator/send', {
      method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({text}),
    }).then(async response => {
      if (!response.ok) {
        const result = await response.json() as {error?: string};
        throw new Error(result.error ?? `发送失败 (${response.status})`);
      }
      // The state poll acknowledges completion of the remote call.
    }).catch(error => finishSend(false, errorMessage(error)));
    return;
  }
  if (value.startsWith('edge_')) {
    void fetch('/api/simulator/action', {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({action: value}),
    }).then(response => { if (!response.ok) throw new Error(`设备 action 失败 (${response.status})`); })
      .catch(error => log('设备 action 错误: ' + errorMessage(error)));
  }
  pageLabel.textContent = value;
  if (value === 'face_online' || value === 'run_node') {
    expressionSelect.value = value === 'run_node' ? 'listening' : 'online';
    applyControls();
  }
}

/** Reads a generated manifest from the local server. */
async function loadManifest(): Promise<Required<Pick<BuildManifest, 'sourceHash'>> & BuildManifest> {
  const response = await fetch('./generated/manifest.json', {cache: 'no-store'});
  if (!response.ok) {
    let detail = '';
    try {
      const build = await request<BuildStatus>('/api/build');
      detail = build.error ?? (build.running ? '构建正在进行，请刷新页面' : '请先运行 npm run build --prefix tools/esp32-editor');
    } catch { /* Preserve the primary missing-artifact error. */ }
    throw new Error(`读取构建清单失败 (${response.status})。${detail}`);
  }
  return validManifest(await response.json() as BuildManifest);
}

/** Updates the build status and reloads a clean editor after a new runtime is published. */
async function updateBuildStatus(): Promise<void> {
  try {
    const status = await request<BuildStatus>('/api/build');
    const manifest = status.manifest;
    buildStatus.textContent = status.running
      ? '正在构建 WebAssembly + ESP32…'
      : status.error
        ? '构建失败：' + status.error
        : status.stale
          ? '底层运行时源码已变化，需要开发者构建'
          : manifest?.firmwareBuilt
            ? `基础运行时 ${manifest.sourceHash?.slice(0, 12) ?? ''}`
            : '预览已构建；固件未验证';
    if (
      sourceHash &&
      manifest?.sourceHash &&
      manifest.sourceHash !== sourceHash &&
      !status.running &&
      !status.error &&
      !window.operitEditor?.isDirty() &&
      !window.operitEditor?.isBusy()
    ) {
      location.reload();
    }
  } catch (error) {
    buildStatus.textContent = errorMessage(error);
  }
}

/** Connects the editor panel to the generated shared LVGL runtime. */
async function initialize(): Promise<void> {
  board = await request<BoardInfo>('/api/board');
  const currentBoard = activeBoard();
  themeSelect.replaceChildren(...currentBoard.themes.map((theme, index) => new Option(theme.name, String(index))));

  const manifest = await loadManifest();
  sourceHash = manifest.sourceHash;
  const module = await import('./generated/ui.mjs?v=' + encodeURIComponent(sourceHash)) as {default: RuntimeFactory};
  if (typeof module.default !== 'function') throw new Error('构建产物缺少 Wasm 工厂函数');
  runtime = await module.default();
  runtime.onAction = handleRuntimeAction;
  if (!runtime._simulator_init()) throw new Error('LVGL 初始化失败');
  applyControls();
  setDeviceState({});
  syncTheme();
  sourceLabel.textContent = `共用源码：${manifest.source ?? 'apps/esp32/lvgl_port/operit_lvgl.c'} / LVGL ${manifest.lvgl ?? '未知'}`;
  log('真实 LVGL WebAssembly 已启动');
  await setupEditor(runtime, log);
  requestAnimationFrame(renderFrame);
}

/** Installs the simulator controls and starts the editor runtime. */
function bindControls(): void {
  canvas.addEventListener('pointerdown', (event: PointerEvent) => {
    pressed = true;
    canvas.focus();
    if (event.isTrusted) canvas.setPointerCapture(event.pointerId);
    touch(event.clientX, event.clientY, true);
    log('touch down');
  });
  canvas.addEventListener('pointermove', (event: PointerEvent) => {
    touch(event.clientX, event.clientY, pressed);
  });
  canvas.addEventListener('pointerup', (event: PointerEvent) => {
    pressed = false;
    touch(event.clientX, event.clientY, false);
    log('touch up');
  });
  canvas.addEventListener('pointercancel', (event: PointerEvent) => {
    pressed = false;
    touch(event.clientX, event.clientY, false);
  });
  canvas.addEventListener('keydown', handleCanvasKeydown);
  query<HTMLButtonElement>('#home').addEventListener('click', () => navigate('home'));
  query<HTMLButtonElement>('#apps').addEventListener('click', () => navigate('apps'));
  const changeTheme = (): void => {
    window.dispatchEvent(new Event('operit-runtime-navigation'));
    activeRuntime()._operit_lvgl_set_theme(Number(themeSelect.value), shapeSelect.value === 'circle');
  };
  themeSelect.addEventListener('change', changeTheme);
  shapeSelect.addEventListener('change', changeTheme);
  for (const input of [wifiInput, edgeInput, expressionSelect]) {
    input.addEventListener('change', () => {
      if (runtime) applyControls();
    });
  }
  gridInput.addEventListener('change', () => {
    grid.style.display = gridInput.checked ? 'block' : 'none';
  });
  query<HTMLButtonElement>('#clear-log').addEventListener('click', () => eventList.replaceChildren());
  colorSelect.addEventListener('change', () => {
    generation = -1;
  });
  query<HTMLButtonElement>('#reset').addEventListener('click', () => location.reload());
  query<HTMLButtonElement>('#capture').addEventListener('click', () => {
    const link = document.createElement('a');
    link.download = 'operit-lvgl-320x240.png';
    link.href = canvas.toDataURL();
    link.click();
  });
}

bindControls();
initialize().catch((error: unknown) => {
  log('ERROR ' + errorMessage(error));
  query<HTMLElement>('.hint').textContent = '预览尚未构建或加载失败，请查看构建状态：' + errorMessage(error);
  editorStatus.textContent = '加载失败：' + errorMessage(error);
});
void updateBuildStatus();
window.setInterval(() => void updateBuildStatus(), 2000);
