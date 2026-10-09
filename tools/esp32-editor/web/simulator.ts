const panel = document.createElement('section');
panel.className = 'simulator-panel';
panel.innerHTML = `<details>
  <summary><span>模拟设备</span><small id="sim-summary">已停止</small></summary>
  <div class="simulator-content">
    <p class="note">开发者连接信息。在 Operit 中发现并添加此模拟设备；聊天操作与硬件共用同一界面。</p>
    <div class="dialog-actions"><button id="sim-start">启动</button><button id="sim-stop">停止</button></div>
    <dl class="simulator-details">
      <div><dt>状态</dt><dd id="sim-status" role="status">模拟设备已停止</dd></div>
      <div><dt>TCP</dt><dd id="sim-address">-</dd></div>
      <div><dt>Edge Token</dt><dd class="sim-token"><input id="sim-token" type="text" readonly spellcheck="false" aria-label="Edge Token"><button id="sim-copy">复制</button></dd></div>
    </dl>
    <details class="simulator-log"><summary>开发者日志</summary><pre id="sim-log"></pre></details>
  </div>
</details>`;
document.querySelector('footer')!.after(panel);
const label = panel.querySelector<HTMLElement>('#sim-status')!;
const summary = panel.querySelector<HTMLElement>('#sim-summary')!;
const address = panel.querySelector<HTMLElement>('#sim-address')!;
const token = panel.querySelector<HTMLInputElement>('#sim-token')!;
let polling = false;
async function refresh(): Promise<void> {
  if (polling) return;
  polling = true;
  try {
    const response = await fetch('/api/simulator/state');
    if (!response.ok) throw new Error(await response.text());
    const state = await response.json();
    token.value = state.token ?? '';
    const connection = state.device?.chat.connected ? '已连接 Space' : state.device?.paired ? '已配对，等待重连' : '等待配对';
    const error = state.device?.error ? ' · ' + state.device.error : '';
    label.textContent = state.ready ? connection + error : state.running ? '正在启动' : '模拟设备已停止';
    summary.textContent = state.ready ? connection : state.running ? '正在启动' : '已停止';
    address.textContent = state.ready ? state.device.address : '-';
    panel.querySelector('#sim-log')!.textContent = state.output;
    // Let the 自绘 UI host reflect the real running session instead of debug toggles.
    window.dispatchEvent(new CustomEvent('operit-simulator-state', {detail: {
      running: state.ready, connected: state.device?.chat.connected === true, paired: state.device?.paired === true,
      pairingCode: state.device?.pairingCode ?? '', spaceState: state.device?.chat.connected ? '已连接 Operit' : '等待连接 Operit',
      spaceJoinPrompt: state.device?.spaceJoinPrompt ?? '', spaceJoinBusy: false,
      spaceJoinRequestId: state.device?.spaceJoinRequestId,
      spaceJoinAssignmentVersion: state.device?.spaceJoinAssignmentVersion,
      chatPreview: state.device?.chatPreview ?? '尚未连接对话',
      plugins: state.device?.plugins, chat: state.device?.chat, chatScreen: state.device?.chatScreen, chatTask: state.device?.chatTask,
      chatSendResult: state.device?.chatSendResult,
    }}));
  } catch (e) { label.textContent = String(e); }
  finally { polling = false; }
}
panel.querySelector('#sim-copy')!.addEventListener('click', () => {
  void navigator.clipboard.writeText(token.value).catch(e => { label.textContent = String(e); });
});
for (const action of ['start', 'stop']) {
  panel.querySelector(`#sim-${action}`)!.addEventListener('click', async () => {
    try {
      const response = await fetch(`/api/simulator/${action}`, {method: 'POST'});
      if (!response.ok) throw new Error(await response.text());
      await refresh();
    } catch (e) { label.textContent = String(e); }
  });
}
void fetch('/api/simulator/start', {method: 'POST'}).then(async response => {
  if (!response.ok) throw new Error(await response.text());
  await refresh();
}).catch(e => { label.textContent = String(e); });
window.setInterval(() => void refresh(), 1000);
