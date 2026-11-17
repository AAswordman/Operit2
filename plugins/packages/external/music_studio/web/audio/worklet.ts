import { copy } from '../../src/shared/model';
import type { Project } from '../../src/shared/model';
import { DspRuntime } from './dsp-runtime';
declare const sampleRate: number;
declare class AudioWorkletProcessor { readonly port: MessagePort; constructor(options?: unknown); }
declare function registerProcessor(name: string, processor: typeof AudioWorkletProcessor): void;
class StudioProcessor extends AudioWorkletProcessor {
  private runtime: DspRuntime;
  private ticks=0;
  private fading: DspRuntime | null = null;
  private fadePosition=0;
  private fadeLength=Math.round(sampleRate*.035);
  private fadeL=new Float32Array(128);
  private fadeR=new Float32Array(128);
  private preview: DspRuntime | null = null;
  private module: WebAssembly.Module;
  private previewL = new Float32Array(128);
  private previewR = new Float32Array(128);
  constructor(options: {processorOptions: {module: WebAssembly.Module}}) {
    super();this.module=options.processorOptions.module;this.runtime=new DspRuntime(options.processorOptions.module,sampleRate);
    this.port.onmessage=({data})=>{
      try {
        if(data.type==='preview-load') {
          this.preview ??= new DspRuntime(this.module,sampleRate);
          this.preview.load(data.project,false);
        }
        else if(data.type==='preview-on')this.preview?.previewOn(data.pitch,data.velocity);
        else if(data.type==='preview-off')this.preview?.previewOff(data.pitch);
        else if(data.type==='preview-clear')this.preview?.api.reset();
        else if(data.type==='update') {
          this.updateRuntime(data.project);
          if(this.preview&&data.previewProject) {
            if(!this.preview.updateProject(data.previewProject)) { this.preview.load(data.previewProject,false); }
          } else if(this.preview&&!data.previewProject)this.preview.api.reset();
        }
        else if(data.type==='parameter') {
          if(!this.runtime.liveParameter(data.trackId,data.group,data.key,data.value,data.effectId)) {
            const project=copy(this.runtime.project),track=project.tracks.find(t=>t.id===data.trackId);
            if(track&&(data.group==='effect'||data.group==='fxparam')) {
              const e=track.effects.find(e=>e.id===data.effectId);if(e){if(data.group==='effect')e.mix=data.value;else e.params[data.key]=data.value;this.updateRuntime(project);}
            }
          }
          this.preview?.liveParameter(data.trackId,data.group,data.key,data.value,data.effectId);
        }
        else if(data.type==='load')this.runtime.load(data.project);
        else if(data.type==='play'){this.runtime.seek(data.beat);this.runtime.playing=true;}
        else if(data.type==='pause'){this.runtime.playing=false;}
        else if(data.type==='stop'){this.runtime.playing=false;this.runtime.seek(data.beat??0);}
        else if(data.type==='seek')this.runtime.seek(data.beat);
        else if(data.type==='metronome')this.runtime.metronome=data.value;
        if(data.id)this.port.postMessage({id:data.id,ok:true});
      }catch(e){this.port.postMessage({id:data.id,error:String(e)});this.runtime.playing=false;}
    };
    this.port.postMessage({ready:true});
  }
  private updateRuntime(project: Project): void {
    if(this.runtime.updateProject(project))return;
    const previous=this.runtime,next=new DspRuntime(this.module,sampleRate);
    next.load(project);next.seek(previous.beat);next.playing=previous.playing;
    if(next.playing)next.resumeHeldNotes();
    this.runtime=next;
    this.fading=previous.playing?previous:null;this.fadePosition=0;
  }
  process(_inputs: Float32Array[][],outputs: Float32Array[][]): boolean {
    const output=outputs[0];if(!output?.[0]||!output[1])return true;
    try {
      this.runtime.process(output[0],output[1]);
      if(this.fading) {
        this.fading.process(this.fadeL,this.fadeR);
        for(let i=0;i<output[0].length;i++) { const mix=Math.min(1,this.fadePosition++/this.fadeLength);output[0][i]=this.fadeL[i]*(1-mix)+output[0][i]*mix;output[1][i]=this.fadeR[i]*(1-mix)+output[1][i]*mix; }
        if(this.fadePosition>=this.fadeLength)this.fading=null;
      }
      if(this.preview) {
        this.preview.processPreview(this.previewL,this.previewR);
        for(let i=0;i<output[0].length;i++) { output[0][i]=Math.max(-1,Math.min(1,output[0][i]+this.previewL[i])); output[1][i]=Math.max(-1,Math.min(1,output[1][i]+this.previewR[i])); }
      }
      if(this.runtime.project&&++this.ticks>=20){this.ticks=0;this.port.postMessage({stats:this.runtime.stats()});}
    }catch(e){output[0].fill(0);output[1].fill(0);this.runtime.playing=false;this.port.postMessage({error:String(e)});}
    return true;
  }
}
registerProcessor('operit-wasm-studio',StudioProcessor);
