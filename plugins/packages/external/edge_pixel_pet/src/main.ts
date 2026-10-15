import { command } from './pet';
ToolPkg.ipc.on('edge.pet', command);
export function registerToolPkg(): boolean { return true; }
/** Configuration editor diagnostics only; the device owns runtime behavior. */
async function diagnostic(): Promise<{
    passed: boolean;
    message: string;
}> {
    const capabilities = await command({ operation: 'inspect' });
    return { passed: capabilities.protocol === 1 && capabilities.input?.local === true,
        message: 'Native scene v1 with device-local interaction' };
}
export function test_connection(): Promise<{
    passed: boolean;
    message: string;
}> { return diagnostic(); }
export function test_tool_call(): Promise<{
    passed: boolean;
    message: string;
}> { return diagnostic(); }
