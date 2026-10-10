import {simulatorStatusLabel, simulatorViewState} from './device-state.js';
import {errorMessage} from './types.js';
import {mountCoreAccess} from './core-access.js';
import type {SimulatorStatus} from './device-state.js';
const panel = document.createElement('section');
panel.className = 'simulator-panel';
panel.innerHTML = `<details>
  <summary><span>ESP32 模拟设备</span><small id="sim-summary">已停止</small></summary>
  <div class="simulator-content">
    <p class="note">页面打开后自动启动 ESP32 模拟设备。可以在下方向 Core 主动申请接入，也可以由 Core 使用设备 TCP 地址和 Token 发起配对。插件 JS 和业务数据保存在 Core；此模拟器验证本板型的呈现与交互。</p>
    <div class="dialog-actions"><button id="sim-start">启动模拟设备</button><button id="sim-stop">停止模拟设备</button></div>
    <dl class="simulator-details">
      <div><dt>状态</dt><dd id="sim-status" role="status">模拟设备已停止</dd></div>
      <div><dt>TCP</dt><dd id="sim-address">-</dd></div>
      <div><dt>Edge Token</dt><dd class="sim-token"><input id="sim-token" type="text" readonly spellcheck="false" aria-label="Edge Token"><button id="sim-copy" disabled>复制</button></dd></div>
    </dl>
    <fieldset><legend>模拟麦克风</legend>
      <p class="note">16 kHz 单声道 PCM，440 Hz 测试音。Core 插件通过 Tools.Edge 打开 sim-pcm，沿已批准的设备空间 TCP 连接接收。此处只配置输入源。</p>
      <label>时长（毫秒）<input id="sim-audio-duration" type="number" min="20" max="10000" step="20" value="1000"></label>
      <label><input id="sim-audio-fail" type="checkbox">结束时模拟采集过载</label>
      <button id="sim-audio-apply" disabled>应用音频设置</button>
      <p id="sim-audio-status" role="status">等待模拟设备</p>
      <p id="sim-audio-error" role="alert"></p>
    </fieldset>
    <details class="simulator-log"><summary>开发者日志</summary><pre id="sim-log"></pre></details>
  </div>
</details>`;
document.querySelector('footer')!.after(panel);
const label = panel.querySelector<HTMLElement>('#sim-status')!;
const summary = panel.querySelector<HTMLElement>('#sim-summary')!;
const address = panel.querySelector<HTMLElement>('#sim-address')!;
const token = panel.querySelector<HTMLInputElement>('#sim-token')!;
const startButton = panel.querySelector<HTMLButtonElement>('#sim-start')!;
const stopButton = panel.querySelector<HTMLButtonElement>('#sim-stop')!;
const copyButton = panel.querySelector<HTMLButtonElement>('#sim-copy')!;
const audioButton = panel.querySelector<HTMLButtonElement>('#sim-audio-apply')!;
const audioStatus = panel.querySelector<HTMLElement>('#sim-audio-status')!;
const audioError = panel.querySelector<HTMLElement>('#sim-audio-error')!;
const updateCoreAccess = mountCoreAccess(panel.querySelector<HTMLElement>('.simulator-content')!);
let audioPending = false;
audioButton.addEventListener('click', async () => {
  audioPending = true; audioButton.disabled = true; audioError.textContent = '';
  try {
    const durationMs = panel.querySelector<HTMLInputElement>('#sim-audio-duration')!.valueAsNumber;
    const fail = panel.querySelector<HTMLInputElement>('#sim-audio-fail')!.checked;
    const response = await fetch('/api/simulator/audio/configure', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({durationMs, fail})});
    await readResponse(response);
    await refresh();
  } catch (error) { audioError.textContent = errorMessage(error); }
  finally { audioPending = false; void refresh(); }
});
let polling = false;
let pendingAction = false;

async function readResponse(response: Response): Promise<SimulatorStatus> {
  const value = await response.json().catch(() => ({})) as SimulatorStatus & {error?: string};
  if (!response.ok) throw new Error(value.error ?? `模拟设备请求失败 (${response.status})`);
  return value;
}
async function refresh(): Promise<void> {
  if (polling) return;
  polling = true;
  try {
    const response = await fetch('/api/simulator/state');
    const state = await readResponse(response);
    token.value = state.token ?? '';
    updateCoreAccess(state.ready === true, state.device?.coreAccess);
    const audio = (state.device as typeof state.device & {audio?: {active: boolean; durationMs: number; blocks: number; byteLength: number; error?: string}})?.audio;
    audioButton.disabled = audioPending || !state.ready || audio?.active === true;
    audioStatus.textContent = audio ? `${audio.active ? '采集中' : audio.error ? '采集失败' : audio.blocks ? '采集已停止' : '尚未采集'} · ${audio.durationMs} ms · ${audio.blocks} 块 · ${audio.byteLength} 字节${audio.error ? ' · ' + audio.error : ''}` : '等待模拟设备';
    label.textContent = simulatorStatusLabel(state);
    summary.textContent = simulatorStatusLabel(state);
    address.textContent = state.ready ? state.device?.address ?? '-' : '-';
    panel.querySelector('#sim-log')!.textContent = state.output ?? '';
    startButton.disabled = pendingAction || state.running === true;
    stopButton.disabled = pendingAction || state.running !== true;
    copyButton.disabled = !token.value;
    // Let the 自绘 UI host reflect the real running session instead of debug toggles.
    window.dispatchEvent(new CustomEvent('operit-simulator-state', {detail: simulatorViewState(state)}));
  } catch (e) { label.textContent = errorMessage(e); summary.textContent = '请求失败'; }
  finally { polling = false; }
}
copyButton.addEventListener('click', () => {
  void navigator.clipboard.writeText(token.value).catch(e => { label.textContent = errorMessage(e); summary.textContent = '请求失败'; });
});
for (const action of ['start', 'stop']) {
  panel.querySelector(`#sim-${action}`)!.addEventListener('click', async () => {
    if (pendingAction) return;
    pendingAction = true;
    startButton.disabled = stopButton.disabled = true;
    try {
      const response = await fetch(`/api/simulator/${action}`, {method: 'POST'});
      await readResponse(response);
      pendingAction = false;
      await refresh();
    } catch (e) { label.textContent = errorMessage(e); summary.textContent = '请求失败'; }
    finally { pendingAction = false; }
  });
}
void fetch('/api/simulator/start', {method: 'POST'}).then(async response => {
  await readResponse(response);
  await refresh();
}).catch(e => { label.textContent = errorMessage(e); summary.textContent = '请求失败'; });
window.setInterval(() => void refresh(), 1000);
