import {packLayout} from '../src/layout/package-layout.mjs';
import {request} from './transport.js';
import {errorMessage, query} from './types.js';
import type {
  DeploySetupOptions,
  DeviceCapabilities,
  FlashState,
  SerialPortsResponse,
} from './types.js';

/** Converts a typed byte array into an ArrayBuffer accepted by Blob and fetch. */
function asArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

/** Starts a browser download for an in-memory file. */
function download(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Installs layout export, device deployment and firmware flashing actions. */
export function setupDeploy({snapshot}: DeploySetupOptions): void {
  const dialog = query<HTMLDialogElement>('#deploy-dialog');
  const feedback = query<HTMLElement>('#deploy-feedback');
  const packageSize = query<HTMLElement>('#package-size');
  const addressInput = query<HTMLInputElement>('#device-address');
  const portSelect = query<HTMLSelectElement>('#flash-port');
  const flashOutput = query<HTMLElement>('#flash-output');
  const resetDataInput = query<HTMLInputElement>('#flash-reset-data');
  const flashRuntimeButton = query<HTMLButtonElement>('#flash-runtime');
  const usbLayoutButton = query<HTMLButtonElement>('#usb-layout');
  let pollingTimer: number | null = null;

  /** Displays a deployment status message. */
  function info(text: string): void {
    feedback.textContent = text;
  }

  /** Stops the active flash-status poll. */
  function stopPolling(): void {
    if (pollingTimer === null) return;
    window.clearInterval(pollingTimer);
    pollingTimer = null;
  }

  /** Polls the firmware flashing process and updates both controls and output. */
  async function pollFlash(): Promise<void> {
    try {
      const state = await request<FlashState>('/api/deploy/flash');
      flashRuntimeButton.disabled = state.running;
      usbLayoutButton.disabled = true;
      flashOutput.textContent = state.error ?? state.output;
      if (!state.running) stopPolling();
    } catch (error) {
      info(errorMessage(error));
    }
  }

  /** Begins polling the firmware flashing process. */
  function startPolling(): void {
    stopPolling();
    pollingTimer = window.setInterval(() => void pollFlash(), 1000);
  }

  /** Opens the deployment dialog with the current draft package size. */
  function openDialog(): void {
    try {
      const bytes = packLayout(snapshot().document).byteLength;
      packageSize.textContent = `当前项目 ${bytes.toLocaleString()} 字节 · 上限 28 KiB · 当前固件不支持动态布局部署`;
      info('可导出当前草稿；当前固定自绘 UI 仅支持固件更新，不会应用布局包。');
    } catch (error) {
      info(errorMessage(error));
    }
    if (!dialog.open) dialog.showModal();
  }

  /** Exports the editable layout JSON. */
  function exportLayout(): void {
    const body = JSON.stringify(snapshot().document, null, 2) + '\n';
    download(new Blob([body], {type: 'application/json'}), 'layout.json');
  }

  /** Exports the binary device layout package. */
  function exportPackage(): void {
    try {
      const bytes = packLayout(snapshot().document);
      download(new Blob([asArrayBuffer(bytes)], {type: 'application/octet-stream'}), 'operit-ui.oui');
      info('已导出布局包，不含固件或编译缓存。');
    } catch (error) {
      info(errorMessage(error));
    }
  }

  /** Checks the device runtime capabilities at the configured address. */
  async function connectDevice(): Promise<void> {
    try {
      const address = addressInput.value;
      const capabilities = await request<DeviceCapabilities>(
        '/api/deploy/device?address=' + encodeURIComponent(address),
      );
      info(
        `已连接 ${capabilities.board} · ${capabilities.dynamicLayout ? "支持动态布局" : "固定自绘 UI，不支持布局部署"} · 协议 ${capabilities.protocol}`,
      );
    } catch (error) {
      info(errorMessage(error));
    }
  }

  /** Loads the available serial ports into the flashing selector. */
  async function refreshPorts(): Promise<void> {
    try {
      const result = await request<SerialPortsResponse>('/api/deploy/ports');
      portSelect.replaceChildren(...result.ports.map((port) => new Option(`${port.port} · ${port.name}`, port.port)));
      info(result.ports.length ? '选择 ESP32 数据线对应串口' : '未发现串口，请连接数据线');
    } catch (error) {
      info(errorMessage(error));
    }
  }

  /** Installs or updates the base ESP32 firmware over USB. */
  async function flashRuntime(): Promise<void> {
    try {
      if (!portSelect.value) throw new Error('请刷新并选择 ESP32 串口');
      const resetData = resetDataInput.checked;
      if (resetData && !window.confirm('将清空设备全部 Flash，删除 Wi-Fi、配对身份、空间状态和布局。需要重新配置和配对，确定继续？')) return;
      flashRuntimeButton.disabled = true;
      await request('/api/deploy/flash', {
        method: 'POST',
        body: {port: portSelect.value, resetData, confirmReset: resetData},
      });
      await pollFlash();
      startPolling();
    } catch (error) {
      info(errorMessage(error));
      flashRuntimeButton.disabled = false;
    }
  }

  query<HTMLButtonElement>('#open-deploy').addEventListener('click', openDialog);
  query<HTMLButtonElement>('#deploy-close').addEventListener('click', () => dialog.close());
  dialog.addEventListener('close', stopPolling);
  query<HTMLButtonElement>('#export-layout').addEventListener('click', exportLayout);
  query<HTMLButtonElement>('#export-package').addEventListener('click', exportPackage);
  query<HTMLButtonElement>('#connect-device').addEventListener('click', () => void connectDevice());
  query<HTMLButtonElement>('#deploy-layout').disabled = true;
  usbLayoutButton.disabled = true;
  query<HTMLButtonElement>('#refresh-ports').addEventListener('click', () => void refreshPorts());
  flashRuntimeButton.addEventListener('click', () => void flashRuntime());
}
