import {errorMessage} from './types.js';
import type {CoreAccessState} from './device-state.js';

export function mountCoreAccess(container: HTMLElement): (ready: boolean, state?: CoreAccessState) => void {
    const panel = document.createElement('fieldset');
    panel.innerHTML = `<legend>向 Core 申请接入</legend>
      <p class="note">模拟设备主动通过现有 TCP 配对并申请加入 Core 的设备空间。配对码由 Core 显示，接入权限在 Core 上批准。</p>
      <label>Core 节点 ID <input data-access="nodeId" type="text" spellcheck="false"></label>
      <label>Core TCP 地址 <input data-access="address" type="text" placeholder="127.0.0.1:端口" spellcheck="false"></label>
      <label>Core Token（局域网可留空）<input data-access="token" type="password" autocomplete="off"></label>
      <button data-access="request">发送接入请求</button>
      <div data-access="confirmation" hidden>
        <label>Core 配对码 <input data-access="code" type="text" inputmode="numeric" maxlength="6" autocomplete="off"></label>
        <button data-access="confirm">确认配对并申请权限</button>
      </div>
      <button data-access="refresh" hidden>刷新申请状态</button>
      <button data-access="cancel" hidden>取消申请</button>
      <p data-access="status" role="status">等待模拟设备</p>
      <p data-access="error" role="alert"></p>`;
    container.insertBefore(panel, container.querySelector('fieldset'));
    const get = <T extends HTMLElement>(name: string): T => panel.querySelector<T>(`[data-access="${name}"]`)!;
    let ready = false, busy = false;
    let state: CoreAccessState = {};
    const render = (): void => {
        const pending = state.request && ['pending', 'approving', 'approved'].includes(state.request.status);
        get('confirmation').hidden = !state.pairing;
        get('refresh').hidden = !state.request;
        get('cancel').hidden = !state.pairing && !pending;
        for (const button of panel.querySelectorAll('button')) button.disabled = !ready || busy;
        get<HTMLButtonElement>('request').disabled = !ready || busy || !!state.pairing || !!pending;
        const labels: Record<string, string> = {pending: '等待 Core 批准', approving: 'Core 正在批准',
            approved: 'Core 已批准，等待同步', joined: '已加入 Core 空间，可以采集音频',
            rejected: 'Core 已拒绝', cancelled: '申请已取消', expired: '申请已过期'};
        get('status').textContent = !ready ? '等待模拟设备' : state.pairing ? '请求已发送，请输入 Core 显示的六位配对码'
            : state.joined ? labels.joined : state.request ? `${labels[state.request.status] ?? state.request.status} · ${state.request.spaceName}`
            : '填写 Core 连接信息后发送请求；已有配对会直接申请权限';
    };
    for (const step of ['request', 'confirm', 'refresh', 'cancel']) {
        get(step).addEventListener('click', async () => {
            if (busy) return;
            busy = true; get('error').textContent = ''; render();
            try {
                const input: Record<string, unknown> = {step};
                if (step === 'request') {
                    input.nodeId = get<HTMLInputElement>('nodeId').value.trim();
                    input.address = get<HTMLInputElement>('address').value.trim();
                    input.token = get<HTMLInputElement>('token').value;
                } else if (step === 'confirm') {
                    input.pairingId = state.pairing?.pairingId;
                    input.confirmationCode = get<HTMLInputElement>('code').value.trim();
                } else if (step === 'refresh') input.requestId = state.request?.requestId;
                const response = await fetch('/api/simulator/core-access', {method: 'POST',
                    headers: {'Content-Type': 'application/json'}, body: JSON.stringify(input)});
                const result = await response.json() as CoreAccessState & {error?: string};
                if (!response.ok) throw new Error(result.error ?? `接入请求失败 (${response.status})`);
                state = result;
                if (!state.pairing) get<HTMLInputElement>('code').value = '';
                get<HTMLInputElement>('token').value = '';
            } catch (error) { get('error').textContent = errorMessage(error); }
            finally { busy = false; render(); }
        });
    }
    return (isReady, snapshot) => {
        ready = isReady;
        if (!busy && snapshot) state = snapshot;
        render();
    };
}
