import {processSpeech} from './processor';
let device: ReturnType<typeof Tools.device> | undefined;
let running: Promise<void> | undefined;
let lastError: string | undefined;
async function command(input: {operation: string; node_id?: string}): Promise<any> {
    if (input.operation === 'status') return {running: !!running, error: lastError};
    if (input.operation === 'stop') { device?.cancel(); return {stopped: true}; }
    if (input.operation !== 'start' || !input.node_id) throw new Error('Select a device node');
    if (running) throw new Error('Stop the current processor first');
    device = Tools.device(input.node_id, {timeoutMs: 600000});
    const channel = await device.transport.open({target: 'operit.device.speech', method: 'results',
        property: 'requests', args: {session: `${Date.now()}-${Math.random()}`}});
    lastError = undefined;
    running = processSpeech(channel, {
        recognize: audio => Tools.Stt.transcribe({audio: Array.from(audio), fileName:'capture.wav', contentType:'audio/wav'}),
        synthesize: text => Tools.Tts.synthesize({text}),
    }).catch(error => { lastError = String(error); }).finally(() => { running = undefined; device = undefined; });
    return {started: true};
}
ToolPkg.ipc.on('device.speech.processor', command);
export function registerToolPkg(): boolean { return true; }
