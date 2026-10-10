export interface CoreAccessState {
    pairing?: {pairingId: string; peerNodeId: string} | null;
    request?: {requestId: string; status: string; spaceName: string; error?: string | null} | null;
    joined?: boolean;
    nodeId?: string;
}

export interface DeviceState {
    coreAccess?: CoreAccessState;
    error?: string;
    address?: string;
    scene?: {
        active: boolean;
        revision: number;
    };
    plugins?: {
        category?: string;
        details?: {
            id?: string;
            description?: string;
            tools?: string[];
            toolOffset?: number;
            toolTotal?: number;
            loading?: boolean;
            error?: string;
        };
        items?: {
            id: string;
            name: string;
            status?: string;
            latencyMs?: number;
            toolStatus?: string;
            toolLatencyMs?: number;
            testError?: string;
        }[];
        offset?: number;
        total?: number;
        loading?: boolean;
        testing?: boolean;
        error?: string | null;
    };
    running?: boolean;
    connected?: boolean;
    paired?: boolean;
    pairingCode?: string;
    spaceState?: string;
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
        conversations?: {
            id: string;
            title: string;
            characterCardName?: string;
        }[];
        error?: string;
    };
    chatSendResult?: {
        ok: boolean;
        error?: string;
    } | null;
}
/** One shared projection for browser events; native scene summaries must reach
 * the renderer just like chat/plugin state. Never include cached bytes or leases. */
export function simulatorViewState(state: {
    ready?: boolean;
    device?: DeviceState | null;
}): DeviceState {
    const device = state.ready === true ? state.device : null;
    return {
        running: state.ready === true, connected: device?.chat?.connected === true, paired: device?.paired === true,
        pairingCode: device?.pairingCode ?? '', spaceState: simulatorConnectionLabel(device),
        spaceJoinPrompt: device?.spaceJoinPrompt ?? '', spaceJoinBusy: device?.spaceJoinBusy ?? false,
        spaceJoinRequestId: device?.spaceJoinRequestId,
        spaceJoinAssignmentVersion: device?.spaceJoinAssignmentVersion,
        chatPreview: device?.chatPreview ?? '尚未连接对话',
        scene: device?.scene,
        plugins: device?.plugins, chat: device?.chat, chatScreen: device?.chatScreen, chatTask: device?.chatTask,
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
    ready?: boolean;
    running?: boolean;
    output?: string;
    token?: string;
    device?: DeviceState | null;
}

export function simulatorStatusLabel(state: SimulatorStatus): string {
    if (!state.ready) return state.running ? '正在启动模拟设备…' : '模拟设备已停止';
    const connection = simulatorConnectionLabel(state.device);
    return state.device?.error ? `${connection} · ${state.device.error}` : connection;
}
