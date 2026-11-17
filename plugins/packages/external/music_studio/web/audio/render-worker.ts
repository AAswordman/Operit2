import { DspRuntime } from './dsp-runtime';
import { encodeWav, analyzeAudio } from './wav';
import type { Project } from '../../src/shared/model';
const scope=self as unknown as {onmessage: ((event:MessageEvent)=>void)|null;postMessage(message:unknown,transfer?:Transferable[]):void};
scope.onmessage=({data}:{data:{module:WebAssembly.Module;project:Project}})=>{
  try {
    const rate=44100,project=data.project;
    const duration=project.bars*project.beatsPerBar*60/project.bpm+3.2;
    const length=Math.ceil(duration*rate),left=new Float32Array(length),right=new Float32Array(length);
    const runtime=new DspRuntime(data.module,rate);runtime.load(project,false);runtime.playing=true;
    const l=new Float32Array(128),r=new Float32Array(128);
    for(let frame=0;frame<length;frame+=128){
      runtime.process(l,r,true);const count=Math.min(128,length-frame);
      left.set(count===128?l:l.subarray(0,count),frame);right.set(count===128?r:r.subarray(0,count),frame);
      if(frame%(128*1024)===0)scope.postMessage({progress:`WASM DSP 渲染 ${Math.round(frame/length*100)}%`});
    }
    let peak=0;for(let i=0;i<length;i++){
      if(!Number.isFinite(left[i])||!Number.isFinite(right[i]))throw Error('DSP 输出非有限数值');
      peak=Math.max(peak,Math.abs(left[i]),Math.abs(right[i]));
    }
    const analysis=analyzeAudio({sampleRate:rate,getChannelData:c=>c===0?left:right},project,runtime.api.max_active());
    const bytes=encodeWav([left,right],rate);
    scope.postMessage({result:{bytes,peak,duration,dropped:runtime.api.dropped(),analysis,backend:'wasm-worker',memoryBytes:runtime.api.memory.buffer.byteLength}},[bytes.buffer]);
  }catch(e){scope.postMessage({error:String(e)});}
};
