import type {FirmwareCheck} from '../src/firmware-check.mts';
export interface CoreAccessState {
    pairing?: {pairingId: string; peerNodeId: string} | null;
    request?: {requestId: string; status: string; spaceName: string; error?: string | null} | null;
    joined?: boolean;
    nodeId?: string;
}

export interface DeviceState {
    sessionId?: string;
    coreAccess?: CoreAccessState;
    error?: string;
    address?: string;
    scene?: {
        active: boolean;
        revision: number;
    };
    speech?: {ready?: boolean; phase?: number; requestId?: string | null; resultId?: number; text?: string | null; error?: string | null};
    playback?: {type: string; id: string; audio?: string; contentType?: string}[];
    running?: boolean;
    connected?: boolean;
    paired?: boolean;
    pairingCode?: string;
    spaceState?: string;
    deviceSpace?: {phase?: number; status?: string; candidates?: string[]; pendingJoin?: boolean};
    spaceJoinPrompt?: string;
    spaceJoinBusy?: boolean;
    spaceJoinRequestId?: string;
    spaceJoinAssignmentVersion?: number;
    chatPreview?: string;
    chatScreen?: string;
    chatTask?: string;
    chat?: {
        connected?: boolean;
        chatId?: string;
        messages?: {
            sender: string;
            text: string;
        }[];
        error?: string;
    };
    chatSendResult?: {
        id: number;
        sendId?: string | null;
        ok: boolean;
        error?: string;
    } | null;
}
/** One shared projection for browser events; native scene summaries must reach
 * the renderer just like chat state. Never include cached bytes or leases. */
export function simulatorViewState(state: {
    ready?: boolean;
    device?: DeviceState | null;
}): DeviceState {
    const device = state.ready === true ? state.device : null;
    return {
        sessionId: device?.sessionId,
        running: state.ready === true, connected: device?.chat?.connected === true, paired: device?.paired === true,
        pairingCode: device?.pairingCode ?? '', spaceState: simulatorConnectionLabel(device),
        spaceJoinPrompt: device?.spaceJoinPrompt ?? '', spaceJoinBusy: device?.spaceJoinBusy ?? false,
        spaceJoinRequestId: device?.spaceJoinRequestId,
        spaceJoinAssignmentVersion: device?.spaceJoinAssignmentVersion,
        deviceSpace: device?.deviceSpace,
        chatPreview: device?.chatPreview ?? '尚未连接对话',
        speech: device?.speech, playback: device?.playback,
        scene: device?.scene,
        chat: device?.chat, chatScreen: device?.chatScreen, chatTask: device?.chatTask,
        chatSendResult: device?.chatSendResult,
    };
}

/** Chat readiness is not evidence of Space membership or data replication. */
export function simulatorConnectionLabel(device?: DeviceState | null): string {
    if (device?.pairingCode) return '等待在 Core 输入配对码';
    if (device?.spaceJoinPrompt) return '等待设备空间审批';
    if (device?.chat?.connected) return 'Core 对话已就绪';
    if (device?.paired) return '已配对，等待 Core 对话';
    return '等待配对';
}

export interface SimulatorStatus {
    firmwareCheck?: FirmwareCheck;
    ready?: boolean;
    running?: boolean;
    output?: string;
    device?: DeviceState | null;
}

export function simulatorStatusLabel(state: SimulatorStatus): string {
    if (state.firmwareCheck?.status === 'checking') return '正在校验 ESP32 固件…';
    if (!state.ready && state.firmwareCheck?.status === 'failed') return '固件检查失败：' + state.firmwareCheck.error;
    if (!state.ready) return state.running ? '正在启动模拟设备…' : '模拟设备已停止';
    const connection = simulatorConnectionLabel(state.device);
    return state.device?.error ? `${connection} · ${state.device.error}` : connection;
}

/** Firmware checks describe this startup, separately from simulated behavior. */
export function firmwareStatusLabel(check?: FirmwareCheck): string {
    if (check?.status === 'checking') return '正在检查真实固件的编译、ABI 与 Flash 分区…';
    if (check?.status === 'passed') return `编译、ABI、分区检查通过 · ${check.imageBytes} / ${check.partitionBytes} 字节`;
    if (check?.status === 'failed') return check.overflowBytes
        ? `应用分区超出 ${check.overflowBytes} 字节 · ${check.imageBytes} / ${check.partitionBytes} 字节`
        : check.error ?? '固件检查失败';
    return '未验证固件，仅用于模拟行为';
}
