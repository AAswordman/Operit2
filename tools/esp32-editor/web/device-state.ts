export interface DeviceState {
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
    const device = state.device;
    return {
        running: state.ready === true, connected: device?.chat?.connected === true, paired: device?.paired === true,
        pairingCode: device?.pairingCode ?? '', spaceState: device?.chat?.connected ? '已连接 Operit' : '等待连接 Operit',
        spaceJoinPrompt: device?.spaceJoinPrompt ?? '', spaceJoinBusy: false,
        spaceJoinRequestId: device?.spaceJoinRequestId,
        spaceJoinAssignmentVersion: device?.spaceJoinAssignmentVersion,
        chatPreview: device?.chatPreview ?? '尚未连接对话',
        scene: device?.scene,
        plugins: device?.plugins, chat: device?.chat, chatScreen: device?.chatScreen, chatTask: device?.chatTask,
        chatSendResult: device?.chatSendResult,
    };
}
