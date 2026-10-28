/** An application protocol owned by this host ToolPkg and the device firmware.
 * The generic transport never inspects these speech messages. */
export interface Channel extends AsyncIterable<{kind: string; value: any}> {
    send(value: any): Promise<void>;
    close(): Promise<void>;
}
export interface SpeechHost {
    recognize(audio: Uint8Array): Promise<{text: string}>;
    synthesize(text: string): Promise<{segments: {audio: number[]; contentType: string}[]}>;
}
const MAX_PCM = 16000 * 2 * 300;
const MAX_TEXT = 16384;
const BLOCK = 4096;
// UTF-8 limits match the device without requiring a Host TextEncoder global.
function textChunks(text: string, limit: number): string[] {
    const chunks: string[] = []; let chunk = '', size = 0, total = 0;
    for (const char of text) {
        const code = char.codePointAt(0)!;
        const bytes = code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
        total += bytes;
        if (total > MAX_TEXT) throw new Error('Speech text exceeds 16384 UTF-8 bytes');
        if (size + bytes > limit) { chunks.push(chunk); chunk = ''; size = 0; }
        chunk += char; size += bytes;
    }
    if (chunk) chunks.push(chunk);
    return chunks;
}
function wav(chunks: Uint8Array[], length: number): Uint8Array {
    const result = new Uint8Array(44 + length), view = new DataView(result.buffer);
    const ascii = (at: number, text: string) => [...text].forEach((c, i) => result[at + i] = c.charCodeAt(0));
    ascii(0, 'RIFF'); view.setUint32(4, length + 36, true); ascii(8, 'WAVE'); ascii(12, 'fmt ');
    view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true);
    view.setUint32(24, 16000, true); view.setUint32(28, 32000, true); view.setUint16(32, 2, true);
    view.setUint16(34, 16, true); ascii(36, 'data'); view.setUint32(40, length, true);
    let offset = 44; for (const chunk of chunks) { result.set(chunk, offset); offset += chunk.length; }
    return result;
}
/** Processes both STT and TTS. Chat sending/sync and screen interaction remain device-owned. */
export async function processSpeech(channel: Channel, host: SpeechHost): Promise<void> {
    let protocol = 1;
    let recording: {id: string; chunks: Uint8Array[]; length: number} | undefined;
    try {
        for await (const event of channel) {
            if (event.kind === 'Completed') break;
            const packet = event.value;
            if (packet?.type === 'ready') {
                if (packet.protocol !== 1 && packet.protocol !== 2) throw new Error('Unsupported device speech protocol');
                protocol = packet.protocol;
                continue;
            }
            const id = packet?.id;
            if (typeof id !== 'string' || !id) throw new Error('Missing device speech request ID');
            try {
                switch (packet.type) {
                    case 'cancel':
                        if (recording?.id === id) recording = undefined;
                        break;
                    case 'stt.begin':
                        if (recording) throw new Error('Recognition is already active');
                        if (packet.sampleRate !== 16000 || packet.channels !== 1 || packet.bitsPerSample !== 16)
                            throw new Error('Unsupported capture format');
                        recording = {id, chunks: [], length: 0};
                        break;
                    case 'stt.data': {
                        if (recording?.id !== id) throw new Error('Recognition session changed');
                        if (!(packet.audio instanceof Uint8Array) || !packet.audio.length || packet.audio.length > BLOCK || packet.audio.length % 2)
                            throw new Error('Invalid PCM block');
                        if (recording.length + packet.audio.length > MAX_PCM) throw new Error('Recognition exceeds 300 seconds');
                        recording.chunks.push(packet.audio.slice()); recording.length += packet.audio.length;
                        break;
                    }
                    case 'stt.end': {
                        if (recording?.id !== id) throw new Error('Recognition session changed');
                        const input = recording; recording = undefined;
                        if (!input.length) throw new Error('No microphone audio received');
                        const result = await host.recognize(wav(input.chunks, input.length));
                        const chunks = textChunks(result.text, 2048);
                        if (protocol === 1) {
                            if (chunks.length > 2) throw new Error('Update device firmware for long recognition results');
                            await channel.send({type:'stt.result', id, text:result.text});
                        } else {
                            for (const text of chunks) await channel.send({type:'stt.text', id, text});
                            await channel.send({type:'stt.result', id, text:''});
                        }
                        break;
                    }
                    case 'tts': {
                        if (typeof packet.text !== 'string' || !packet.text.trim()) throw new Error('TTS text is empty');
                        let index = 0;
                        for (const text of textChunks(packet.text, 1024)) {
                            const result = await host.synthesize(text);
                            if (!result.segments.length) throw new Error('TTS returned no audio');
                            for (const segment of result.segments) {
                                const audio = Uint8Array.from(segment.audio);
                                if (!audio.length) throw new Error('TTS returned empty audio');
                                await channel.send({type:'tts.segment', id, index:index++, contentType:segment.contentType});
                                for (let offset = 0; offset < audio.length; offset += BLOCK)
                                    await channel.send({type:'tts.data', id, audio:audio.slice(offset, offset + BLOCK)});
                            }
                        }
                        await channel.send({type:'tts.end', id});
                        break;
                    }
                    default: throw new Error('Unsupported device speech request');
                }
            } catch (failure) {
                if (recording?.id === id) recording = undefined;
                await channel.send({type:'error', id, error: String(failure).slice(0,256)});
            }
        }
    } finally { recording = undefined; await channel.close(); }
}
