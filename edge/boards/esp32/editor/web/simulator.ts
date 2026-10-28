import {simulatorStatusLabel, simulatorViewState} from './device-state.js';
import {errorMessage} from './types.js';
import {mountCoreAccess} from './core-access.js';
import type {SimulatorStatus} from './device-state.js';
const panel = document.createElement('section');
panel.className = 'simulator-panel';
panel.innerHTML = `<details>
  <summary><span>ESP32 模拟设备</span><small id="sim-summary">已停止</small></summary>
  <div class="simulator-content">
    <p class="note">页面打开后自动启动 ESP32 模拟设备。可以在下方向 Core 主动申请接入，也可以由 Core 使用设备 TCP 地址发起配对并输入设备码。插件 JS 和业务数据保存在 Core；此模拟器验证本板型的呈现与交互。</p>
    <div class="dialog-actions"><button id="sim-start">启动模拟设备</button><button id="sim-stop">停止模拟设备</button></div>
    <dl class="simulator-details">
      <div><dt>状态</dt><dd id="sim-status" role="status">模拟设备已停止</dd></div>
      <div><dt>TCP</dt><dd id="sim-address">-</dd></div>
    </dl>
    <details class="simulator-log"><summary>开发者日志</summary><pre id="sim-log"></pre></details>
  </div>
</details>`;
document.querySelector('footer')!.after(panel);
const updateCoreAccess = mountCoreAccess(panel.querySelector('.simulator-content')!);
const label = panel.querySelector<HTMLElement>('#sim-status')!;
const summary = panel.querySelector<HTMLElement>('#sim-summary')!;
const address = panel.querySelector<HTMLElement>('#sim-address')!;
const startButton = panel.querySelector<HTMLButtonElement>('#sim-start')!;
const stopButton = panel.querySelector<HTMLButtonElement>('#sim-stop')!;
let polling = false;
let refreshRequested = false;
let pendingAction = false;

async function readResponse(response: Response): Promise<SimulatorStatus> {
  const value = await response.json().catch(() => ({})) as SimulatorStatus & {error?: string};
  if (!response.ok) throw new Error(value.error ?? `模拟设备请求失败 (${response.status})`);
  return value;
}
async function refresh(): Promise<void> {
  if (polling) { refreshRequested = true; return; }
  polling = true;
  try {
    const response = await fetch('/api/simulator/state');
    const state = await readResponse(response);
    updateCoreAccess(state.ready === true, state.device?.coreAccess);
    label.textContent = simulatorStatusLabel(state);
    summary.textContent = simulatorStatusLabel(state);
    address.textContent = state.ready ? state.device?.address ?? '-' : '-';
    panel.querySelector('#sim-log')!.textContent = state.output ?? '';
    startButton.disabled = pendingAction || state.running === true;
    stopButton.disabled = pendingAction || state.running !== true;
    // Let the 自绘 UI host reflect the real running session instead of debug toggles.
    window.dispatchEvent(new CustomEvent('operit-simulator-state', {detail: simulatorViewState(state)}));
  } catch (e) { label.textContent = errorMessage(e); summary.textContent = '请求失败'; }
  finally {
    polling = false;
    if (refreshRequested) { refreshRequested = false; void refresh(); }
  }
}
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
window.addEventListener('operit-simulator-refresh', () => void refresh());
window.setInterval(() => void refresh(), 250);
