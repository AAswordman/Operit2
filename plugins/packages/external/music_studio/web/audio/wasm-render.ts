import { type Project,LIMITS,copy } from '../../src/shared/model';
import { parseProject } from '../../src/shared/validation';
import { wasmModule } from './wasm-loader';
import type { AudioAnalysis } from './wav';
declare const __RENDER_WORKER_SOURCE__:string;
interface Result {bytes:Uint8Array;peak:number;duration:number;dropped:number;analysis:AudioAnalysis;backend:string;memoryBytes:number}
let busy=false;
export async function renderWav(raw:Project,progress:(s:string)=>void=()=>undefined):Promise<Result>{
  if(busy)throw Error('已有导出任务运行中');
  const project=parseProject(copy(raw));
  if(project.bars*project.beatsPerBar*60/project.bpm>LIMITS.renderSeconds)throw Error(`离线渲染限 ${LIMITS.renderSeconds} 秒`);
  busy=true;let worker:Worker|undefined;let timer:ReturnType<typeof setTimeout>|undefined;
  const url=URL.createObjectURL(new Blob([__RENDER_WORKER_SOURCE__],{type:'text/javascript'}));
  try{
    progress('编译 WASM DSP…');const module=await wasmModule();worker=new Worker(url);
    return await new Promise<Result>((resolve,reject)=>{
      timer=setTimeout(()=>reject(Error('WASM 导出超时，工作线程已终止')),180000);
      worker!.onerror=e=>reject(Error(e.message||'导出线程失败'));
      worker!.onmessage=({data})=>{if(data.error)reject(Error(data.error));else if(data.progress)progress(data.progress);else if(data.result)resolve(data.result);};
      worker!.postMessage({module,project});
    });
  }finally{if(timer)clearTimeout(timer);worker?.terminate();URL.revokeObjectURL(url);busy=false;}
}
