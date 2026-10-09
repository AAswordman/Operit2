import type { EdgeEventHookEvent, EdgeEventAck } from '../../../../types/edge';
import { command, onEdge } from './pet';
ToolPkg.ipc.on('edge.pet', command);
export function registerToolPkg(): boolean { return true; }
/** One generic listener; this plugin's action handlers consume display.scene events. */
export function on_edge_event(event: EdgeEventHookEvent): Promise<EdgeEventAck> {
    return onEdge(event.eventPayload);
}
/** Existing Core diagnostic exports: capability read only, no display lease. */
async function diagnostic(): Promise<{
    passed: boolean;
    message: string;
}> {
    const capabilities = await command({ operation: 'test' });
    return { passed: capabilities.protocol === 1 && capabilities.events?.push === true,
        message: 'Edge scene v1 and event delivery capabilities' };
}
export function test_connection(): Promise<{
    passed: boolean;
    message: string;
}> { return diagnostic(); }
export function test_tool_call(): Promise<{
    passed: boolean;
    message: string;
}> { return diagnostic(); }
