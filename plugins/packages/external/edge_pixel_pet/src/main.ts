import { command, onScene, type ScenePayload } from './pet';
ToolPkg.ipc.on('edge.pet', command);
export function registerToolPkg(): boolean { return true; }
/** Native event ingress invokes only this fixed existing-runtime export. */
export function on_edge_scene_event(event: {
    eventPayload: ScenePayload;
}): Promise<any> {
    return onScene(event.eventPayload);
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
