import { type Project } from '../../src/shared/model';
import { auditionProject } from './audition';
import { parseProject } from '../../src/shared/validation';
import { wasmModule, WASM_SHA256 } from './wasm-loader';
declare const __WORKLET_SOURCE__: string;
type Stats={backend:string;beat:number;playing:boolean;voices:number;dropped:number;maxVoices:number;memoryBytes:number;blocks:number;wraps:number;trackPeaks:number[]};
export class WasmAudioEngine {
  context: AudioContext|null=null;
  project: Project;
  playing=false;
  unlocked=false;
  onEnded: (()=>void)|null=null;
  private node: AudioWorkletNode|null=null;
  private analyser: AnalyserNode|null=null;
  private split: ChannelSplitterNode|null=null;
  private channels: AnalyserNode[]=[];
  private wave=new Float32Array(1024);
  private spectrumData=new Uint8Array(512);
  private channelData=new Float32Array(256);
  private position=0;
  private click=false;
  private generation=0;
  private initialized: Promise<void>|null=null;
  private id=0;
  private pending=new Map<number,{resolve():void;reject(e:Error):void;timer:ReturnType<typeof setTimeout>}>();
  private latest: Stats|null=null;
  private failure: Error|null=null;
  private previewTrackId='';
  private previewKey = '';
  private previewSerial = 0;
  private previewNotes = new Map<number,number>();
  get auditioning(): boolean { return this.previewNotes.size > 0; }
  get auditionPitches(): number[] { return [...this.previewNotes.keys()]; }
  async previewNoteOn(trackId: string, pitch: number, velocity = .75): Promise<void> {
    const token=++this.previewSerial; this.previewNotes.set(pitch,token);
    try {
      await this.unlock();
      if(this.previewNotes.get(pitch)!==token)return; // Released before audio initialization finished.
      const key=`${this.project.id}:${this.project.revision}:${trackId}`;
      if(key!==this.previewKey) { this.previewTrackId=trackId; this.previewKey=key; this.node!.port.postMessage({ type:'preview-load',project:auditionProject(this.project,trackId) }); }
      await this.message('preview-on',{ pitch,velocity });
    } catch(error) { if(this.previewNotes.get(pitch)===token)this.previewNotes.delete(pitch); throw error; }
  }
  previewNoteOff(pitch: number): void { this.previewNotes.delete(pitch); this.node?.port.postMessage({ type:'preview-off',pitch }); }
  previewClear(): void { this.previewNotes.clear(); this.node?.port.postMessage({ type:'preview-clear' }); }

  constructor(project:Project){this.project=parseProject(project);}
  get beat():number{return this.position;}
  get activeVoices():number{return this.playing?(this.latest?.voices??0):0;}
  get dropped():number{return this.latest?.dropped??0;}
  get metronome():boolean{return this.click;}
  set metronome(value:boolean){this.click=value;this.node?.port.postMessage({type:'metronome',value});}
  diagnostics():object{return {...this.latest,backend:'wasm-audio-worklet',wasmSha256:WASM_SHA256,audioNodes:this.node?5:0,error:this.failure?.message??null};}
  private message(type:string,extra:Record<string,unknown>={}):Promise<void>{
    if(!this.node)return Promise.reject(Error('WASM 音频未初始化'));
    const id=++this.id;
    return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{this.pending.delete(id);reject(Error('音频线程响应超时'));},10000);this.pending.set(id,{resolve,reject,timer});this.node!.port.postMessage({type,id,...extra});});
  }
  async unlock():Promise<void>{
    if(!this.context){
      if(!window.isSecureContext||!window.AudioWorkletNode)throw Error('需要支持 AudioWorklet 的安全 WebView（HTTPS 或 localhost），不会退回不稳定的旧引擎');
      this.context=new AudioContext({latencyHint:'interactive'});
      this.context.onstatechange=()=>{this.unlocked=this.context?.state==='running';};
    }
    await this.context.resume();this.unlocked=this.context.state==='running';
    if(!this.unlocked)throw Error('请点击播放以启用音频');
    if(!this.initialized)this.initialized=this.initialize().catch(e=>{this.initialized=null;throw e;});
    await this.initialized;
    if(this.failure)throw this.failure;
  }
  private async initialize():Promise<void>{
    const ctx=this.context!;
    const module=await wasmModule();
    const url=URL.createObjectURL(new Blob([__WORKLET_SOURCE__],{type:'text/javascript'}));
    try{await ctx.audioWorklet.addModule(url);}finally{URL.revokeObjectURL(url);}
    this.node=new AudioWorkletNode(ctx,'operit-wasm-studio',{numberOfInputs:0,numberOfOutputs:1,outputChannelCount:[2],processorOptions:{module}});
    this.node.onprocessorerror=()=>{this.failure=Error('WASM 音频线程异常，已停止播放');this.playing=false;this.onEnded?.();};
    this.node.port.onmessage=({data})=>{
      if(data.id){const pending=this.pending.get(data.id);if(pending){clearTimeout(pending.timer);this.pending.delete(data.id);if(data.error)pending.reject(Error(data.error));else pending.resolve();}}
      if(data.error){this.failure=Error(data.error);this.playing=false;this.onEnded?.();}
      if(data.stats){this.latest=data.stats;if(this.playing){this.position=data.stats.beat;if(!data.stats.playing){this.playing=false;this.onEnded?.();}}}
    };
    this.analyser=ctx.createAnalyser();this.analyser.fftSize=1024;
    this.split=ctx.createChannelSplitter(2);this.channels=[ctx.createAnalyser(),ctx.createAnalyser()];
    this.node.connect(this.analyser).connect(ctx.destination);this.node.connect(this.split);
    this.channels.forEach((a,i)=>{a.fftSize=256;this.split!.connect(a,i);});
    await this.message('load',{project:this.project});
    this.metronome=this.click;
  }
  /** Gesture preview is small and realtime; committed projects still go through durable validation. */
  previewParameter(trackId: string | undefined,group: string,key: string,value: number,effectId?: string): void {
    this.node?.port.postMessage({ type:'parameter',trackId,group,key,value,effectId });
  }
  setProject(project:Project):void{
    const next=parseProject(project),same=this.project.id===next.id;
    if(same){
      this.project=next;
      if(this.node)void this.message('update',{ project:next,previewProject:this.previewTrackId&&next.tracks.some(t=>t.id===this.previewTrackId)?auditionProject(next,this.previewTrackId):null }).catch(e=>{this.failure=e;});
      if(this.previewTrackId)this.previewKey=`${next.id}:${next.revision}:${this.previewTrackId}`;
      return;
    }
    const wasPlaying=this.playing;this.previewClear();this.previewKey='';this.previewTrackId='';this.pause();this.project=next;this.position=0;
    if(this.node)void this.message('load',{project:next}).then(()=>{if(wasPlaying)return this.play();}).catch(e=>{this.failure=e;});
  }

  async play():Promise<void>{
    if(this.playing)return;const generation=++this.generation;
    await this.unlock();if(generation!==this.generation)return;
    if(this.project.loop.enabled&&(this.position<this.project.loop.start||this.position>=this.project.loop.end))this.position=this.project.loop.start;
    if(this.position>=this.project.bars*this.project.beatsPerBar)this.position=0;
    await this.message('play',{beat:this.position});if(generation!==this.generation)return;this.playing=true;
  }
  pause():void{this.generation++;this.playing=false;this.node?.port.postMessage({type:'pause'});}
  stop():void{this.pause();this.position=this.project.loop.enabled?this.project.loop.start:0;this.node?.port.postMessage({type:'stop',beat:this.position});}
  seek(beat:number):void{this.position=Math.max(0,Math.min(beat,this.project.bars*this.project.beatsPerBar));this.node?.port.postMessage({type:'seek',beat:this.position});}
  peak(trackId?:string):number{
    if(!this.playing && !this.auditioning)return 0;
    if(trackId)return this.latest?.trackPeaks[this.project.tracks.findIndex(t=>t.id===trackId)]??0;
    this.analyser?.getFloatTimeDomainData(this.wave);let peak=0;for(const value of this.wave)peak=Math.max(peak,Math.abs(value));return peak;
  }
  stereoPeak():[number,number]{
    if(!this.playing && !this.auditioning)return [0,0];const peaks:[number,number]=[0,0];
    this.channels.forEach((a,i)=>{a.getFloatTimeDomainData(this.channelData);for(const v of this.channelData)peaks[i]=Math.max(peaks[i],Math.abs(v));});return peaks;
  }
  waveform():Float32Array{if(this.playing || this.auditioning)this.analyser?.getFloatTimeDomainData(this.wave);else this.wave.fill(0);return this.wave;}
  spectrum():Uint8Array{if(this.playing || this.auditioning)this.analyser?.getByteFrequencyData(this.spectrumData);else this.spectrumData.fill(0);return this.spectrumData;}
  async dispose():Promise<void>{this.previewClear();this.pause();for(const p of this.pending.values()){clearTimeout(p.timer);p.reject(Error('音频已关闭'));}this.pending.clear();this.node?.port.close();this.node?.disconnect();this.analyser?.disconnect();this.split?.disconnect();for(const a of this.channels)a.disconnect();await this.context?.close();this.context=null;this.node=null;}
}
