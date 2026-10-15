import {SpeechPcm} from './device-pcm.js';
import type {DeviceState} from './device-state.js';
async function rpc(input: Record<string, unknown>): Promise<any> {
    const response = await fetch('/api/simulator/speech', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(input)});
    const result = await response.json();
    if (!response.ok) throw new Error(result.error ?? '语音请求失败');
    window.dispatchEvent(new Event('operit-simulator-refresh'));
    return result;
}
/** Browser is this simulated device's microphone and speaker driver. */
export class DeviceSpeech {
    private requestId?: string;
    private sessionId?: string;
    private capture?: MediaStream;
    private context?: AudioContext;
    private source?: MediaStreamAudioSourceNode;
    private processor?: ScriptProcessorNode;
    private pcm?: SpeechPcm;
    private queue: Uint8Array[] = [];
    private sending?: Promise<void>;
    private starting = false;
    private playback?: {id: string; type: string; chunks: Uint8Array[]; length: number};
    private audio = new Audio();
    private audioUrls: string[] = [];
    private currentUrl?: string;
    private report: (message: string) => void;
    constructor(report: (message: string) => void) {
        this.report=report;
        this.audio.onended = () => this.playNext();
        this.audio.onerror = () => { this.report('合成音频播放失败'); this.playNext(); };
    }
    async action(): Promise<void> {
        if (this.starting) return;
        this.starting = true;
        try {
            if (this.requestId) {
                const id = this.requestId;
                this.processor?.disconnect(); this.source?.disconnect(); this.capture?.getTracks().forEach(t=>t.stop());
                const tail = this.pcm?.flush(); if (tail?.length) this.queue.push(tail);
                await this.drain(); this.releaseCapture();
                await rpc({operation:'finish', requestId:id}); this.report('正在识别…');
            } else {
                const context = new AudioContext(); this.context = context;
                await context.resume();
                const result = await rpc({operation:'begin'}); this.requestId = result.requestId;
                const capture = await navigator.mediaDevices.getUserMedia({audio:true});
                this.capture = capture; this.pcm = new SpeechPcm(context.sampleRate);
                this.source = context.createMediaStreamSource(capture);
                this.processor = context.createScriptProcessor(1024,1,1);
                this.processor.onaudioprocess = event => {
                    if (!this.pcm || !this.requestId) return;
                    this.queue.push(...this.pcm.push(event.inputBuffer.getChannelData(0)));
                    if (this.queue.length > 16) { void this.fail('麦克风传输跟不上采集速度'); return; }
                    void this.drain().catch(error=>this.fail(String(error)));
                };
                this.source.connect(this.processor); this.processor.connect(context.destination);
                this.report('录音中，再次点击结束');
            }
        } catch (error) { await this.fail(String(error)); }
        finally { this.starting = false; }
    }
    private async drain(): Promise<void> {
        if (this.sending) { await this.sending; if (!this.queue.length) return; }
        this.sending = (async()=>{
            while(this.queue.length && this.requestId) {
                const bytes = this.queue.shift()!;
                let binary = ''; bytes.forEach(b=>binary+=String.fromCharCode(b));
                await rpc({operation:'pcm', requestId:this.requestId, audio:btoa(binary)});
            }
        })();
        try { await this.sending; } finally { this.sending = undefined; }
    }
    private async fail(message: string): Promise<void> {
        const id = this.requestId; this.releaseCapture(); this.report(message);
        if (id) await rpc({operation:'cancel', requestId:id, error:message.slice(0,256)}).catch(()=>{});
    }
    private releaseCapture(): void {
        this.processor?.disconnect(); this.source?.disconnect(); this.capture?.getTracks().forEach(t=>t.stop());
        void this.context?.close(); this.processor=undefined;this.source=undefined;this.capture=undefined;
        this.context=undefined; this.pcm=undefined; this.requestId=undefined; this.queue=[];
    }
    async speak(text: string): Promise<void> {
        try { await rpc({operation:'tts',text}); this.report('正在合成语音…'); }
        catch(error) { this.report(String(error)); }
    }
    update(state: DeviceState): void {
        if (state.sessionId !== this.sessionId) {
            this.releaseCapture(); this.playback=undefined; this.audio.pause();
            if(this.currentUrl) URL.revokeObjectURL(this.currentUrl); this.currentUrl=undefined;
            this.audioUrls.forEach(url=>URL.revokeObjectURL(url));this.audioUrls=[];this.sessionId=state.sessionId;
        }
        if (this.requestId && state.speech?.requestId !== this.requestId && !this.starting) this.releaseCapture();
        for (const packet of state.playback ?? []) {
            if (packet.type === 'tts.segment') { this.finishSegment(); this.playback={id:packet.id,type:packet.contentType??'audio/wav',chunks:[],length:0}; }
            else if(packet.type === 'tts.data' && this.playback?.id === packet.id && packet.audio) {
                const bytes = Uint8Array.from(atob(packet.audio),c=>c.charCodeAt(0));this.playback.chunks.push(bytes);this.playback.length+=bytes.length;
                if(this.playback.length>16*1024*1024) { this.playback=undefined;this.report('合成音频超出播放容量'); }
            } else if(packet.type === 'tts.end') this.finishSegment();
        }
    }
    private finishSegment(): void {
        if(!this.playback) return;
        const segment=this.playback;this.playback=undefined;
        if(!segment.length) return;
        this.audioUrls.push(URL.createObjectURL(new Blob(segment.chunks as BlobPart[],{type:segment.type})));
        if(!this.currentUrl) this.playNext();
    }
    private playNext(): void {
        if(this.currentUrl) URL.revokeObjectURL(this.currentUrl);
        this.currentUrl=this.audioUrls.shift();
        if(!this.currentUrl) return;
        this.audio.src=this.currentUrl;
        void this.audio.play().catch(error=>{this.report(`音频播放失败：${String(error)}`);this.playNext();});
    }
}
